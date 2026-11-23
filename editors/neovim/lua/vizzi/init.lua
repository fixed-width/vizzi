local api = vim.api
local uv = vim.uv
local origin = require('vizzi.origin')
local M = {}

local active
local pending
local leaving = false
local start

local function report(failure)
  vim.notify('Vizzi: ' .. tostring(failure), vim.log.levels.ERROR)
end

local function cancel_timer(timer)
  if timer and not timer:is_closing() then
    timer:close()
  end
end

local function present(session)
  if session.process:is_closing() then
    return
  end
  local ok, browser = pcall(vim.ui.open, session.url)
  if not ok or not browser then
    vim.notify('Vizzi is running. Open ' .. session.url .. ' in your browser.', vim.log.levels.WARN)
  else
    vim.notify('Vizzi: ' .. session.url)
  end
end

local function retire(session)
  if session.stopping then
    return
  end
  session.stopping = true
  session.origin.close()
  cancel_timer(session.startup_timer)
  cancel_timer(session.window_timer)
  if not session.process:is_closing() then
    session.process:kill('sigint')
    session.stop_timer = vim.defer_fn(function()
      if not session.process:is_closing() then
        session.process:kill('sigkill')
      end
    end, 2000)
  end
end

local function start_pending()
  local intent = pending
  pending = nil
  if intent and not leaving and api.nvim_win_is_valid(intent.window) then
    local ok, failure = pcall(start, intent)
    if not ok then
      report(failure)
    end
  end
end

start = function(intent)
  local session = {
    root = intent.root,
    window = intent.window,
    origin = origin.start(intent.root, intent.window),
  }
  local args = { intent.executable, '--root', intent.root, '--editor-connection', session.origin.file }
  if intent.config then
    vim.list_extend(args, { '--config', intent.config })
  else
    vim.list_extend(args, { '--port', '0' })
  end
  local diagnostic = ''
  local ready = false
  local function stderr(_, data)
    if not data or session.stopping then
      return
    end
    diagnostic = (diagnostic .. data):sub(-8192)
    if ready then
      return
    end
    local port = tonumber(('\n' .. diagnostic):match('\nVizzi: http://127%.0%.0%.1:(%d+)\r?\n'))
    if port and port > 0 and port <= 65535 then
      ready = true
      vim.schedule(function()
        if active == session and not session.stopping then
          if not api.nvim_win_is_valid(session.window) then
            retire(session)
            return
          end
          cancel_timer(session.startup_timer)
          session.url = ('http://127.0.0.1:%d/'):format(port)
          present(session)
        end
      end)
    end
  end
  local function on_exit(result)
    session.origin.close()
    cancel_timer(session.startup_timer)
    cancel_timer(session.stop_timer)
    cancel_timer(session.window_timer)
    if active == session then
      active = nil
      if not session.stopping then
        local details = vim.trim(diagnostic)
        report(('process exited (%d)%s'):format(result.code, details == '' and '' or '\n' .. details))
      end
      start_pending()
    end
  end
  local ok, process = pcall(vim.system, args, {
    cwd = intent.root,
    stdout = false,
    stderr = stderr,
  }, vim.schedule_wrap(on_exit))
  if not ok then
    session.origin.close()
    error(process, 0)
  end
  session.process = process
  active = session
  -- Non-nested autocommands can close a split without emitting WinClosed.
  session.window_timer = uv.new_timer()
  session.window_timer:start(500, 500, vim.schedule_wrap(function()
    if active == session and not api.nvim_win_is_valid(session.window) then
      retire(session)
    end
  end))
  session.startup_timer = vim.defer_fn(function()
    retire(session)
    report('workspace startup timed out\n' .. vim.trim(diagnostic))
  end, 15000)
end

local function enqueue(directory)
  local path = directory and vim.fn.fnamemodify(directory, ':p') or vim.fn.getcwd()
  local root = uv.fs_realpath(path)
  local stat = root and uv.fs_stat(root)
  assert(stat and stat.type == 'directory', 'Choose an existing project directory')
  if active and not active.stopping and active.root == root and not active.process:is_closing()
    and api.nvim_win_is_valid(active.window) then
    if active.url then
      present(active)
    end
    return
  end
  local executable = vim.fn.exepath(vim.g.vizzi_executable or 'vizzi')
  assert(executable ~= '', 'Put vizzi on PATH or set vim.g.vizzi_executable')
  local config = vim.g.vizzi_config
  pending = {
    root = root,
    window = api.nvim_get_current_win(),
    executable = executable,
    config = config and vim.fn.fnamemodify(config, ':p') or nil,
  }
  if active then
    retire(active)
  else
    start_pending()
  end
end

function M.open(directory)
  local ok, failure = pcall(enqueue, directory)
  if not ok then
    report(failure)
  end
end

function M.stop()
  pending = nil
  if active then
    retire(active)
  end
end

local group = api.nvim_create_augroup('VizziWorkspace', { clear = true })
api.nvim_create_autocmd('WinClosed', {
  group = group,
  callback = function(event)
    local window = tonumber(event.match)
    if pending and pending.window == window then
      pending = nil
    end
    if active and active.window == window then
      retire(active)
    end
  end,
})
api.nvim_create_autocmd('VimLeavePre', {
  group = group,
  callback = function()
    leaving = true
    M.stop()
    if active then
      active.process:wait(2000)
    end
  end,
})

return M
