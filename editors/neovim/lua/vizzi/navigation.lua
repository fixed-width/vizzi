local api = vim.api
local uv = vim.uv
local sha256 = require('vizzi.sha256')
local M = {}

local changed = 'Saved source changed or does not match the captured source'
local unavailable = 'The verified target file is unavailable'
local expired = 'The originating editor request expired or closed'
local bom = '\239\187\191'

local function same_file(before, after)
  return after and after.type == 'file' and before.dev == after.dev and before.ino == after.ino
    and before.size == after.size
    and before.mtime.sec == after.mtime.sec and before.mtime.nsec == after.mtime.nsec
    and before.ctime.sec == after.ctime.sec and before.ctime.nsec == after.ctime.nsec
end

local function read_source(request, callback)
  local complete = vim.schedule_wrap(callback)
  local expected = request.source.bytes
  local function opened(err, file)
    if err then
      complete(nil, unavailable)
      return
    end
    local stat = uv.fs_fstat(file)
    local function read_done(read_error, bytes)
      uv.fs_close(file)
      if read_error or not bytes or #bytes ~= expected then
        complete(nil, changed)
      else
        complete(bytes, nil, stat)
      end
    end
    if not stat or stat.type ~= 'file' or stat.size ~= expected then
      read_done(changed)
      return
    end
    if not uv.fs_read(file, expected + 1, 0, read_done) then
      read_done(unavailable)
    end
  end
  local flags = uv.constants.O_RDONLY + uv.constants.O_NONBLOCK
  if uv.fs_realpath(request.path) ~= request.path then
    complete(nil, changed)
  elseif not uv.fs_open(request.path, flags, 0, opened) then
    complete(nil, unavailable)
  end
end

local function buffer_matches(buffer, path)
  if not api.nvim_buf_is_loaded(buffer) then
    return false
  end
  local name = api.nvim_buf_get_name(buffer)
  return name ~= '' and uv.fs_realpath(name) == path
end

local function verify_buffer(buffer, bytes)
  local options = vim.bo[buffer]
  if options.modified then
    return 'The target buffer has unsaved changes; save it before returning from Vizzi'
  end
  local count = api.nvim_buf_line_count(buffer)
  if options.buftype ~= '' or api.nvim_buf_get_offset(buffer, count) > #bytes + 1 then
    return 'The target editor buffer differs from captured source'
  end
  local separator = options.fileformat == 'dos' and '\r\n' or '\n'
  local text = table.concat(api.nvim_buf_get_lines(buffer, 0, -1, true), separator)
  local represented = bytes:sub(1, 3) == bom and bytes:sub(4) or bytes
  if options.endofline and (text ~= '' or represented ~= '') then
    text = text .. separator
  end
  if text ~= represented and text ~= bytes then
    return 'The target editor buffer differs from captured source'
  end
end

local function checked_target(path, bytes)
  local target
  local buffers = api.nvim_list_bufs()
  if #buffers > 8192 then
    return nil, 'The editor buffer allowance was exceeded'
  end
  for _, buffer in ipairs(buffers) do
    if buffer_matches(buffer, path) then
      local problem = verify_buffer(buffer, bytes)
      if problem then
        return nil, problem
      end
      target = target or buffer
    end
  end
  return target
end

local function show_target(window, target)
  local outgoing = api.nvim_win_get_buf(window)
  if outgoing == target then
    return
  end
  local options = vim.bo[outgoing]
  local bufhidden = options.bufhidden
  local protect = options.modified and bufhidden ~= '' and bufhidden ~= 'hide'
  if protect then
    options.bufhidden = 'hide'
  end
  local ok, failure = pcall(api.nvim_win_call, window, function()
    vim.cmd.buffer({ args = { tostring(target) }, mods = { hide = true } })
  end)
  if protect and api.nvim_buf_is_valid(outgoing) and options.bufhidden == 'hide' then
    options.bufhidden = bufhidden
  end
  if not ok then
    error(failure, 0)
  end
end

function M.open(window, request, alive, reply)
  local function verify(proceed)
    read_source(request, function(bytes, problem, stat)
      if not alive() or not api.nvim_win_is_valid(window) then
        reply(expired)
        return
      end
      if not bytes then
        reply(problem)
        return
      end
      local hashed, digest = pcall(sha256, bytes)
      if not hashed then
        reply('Neovim could not verify the saved bytes of the target')
        return
      end
      if digest ~= request.source.sha256 or uv.fs_realpath(request.path) ~= request.path
        or not same_file(stat, uv.fs_lstat(request.path)) then
        reply(changed)
        return
      end
      local ok, failure = pcall(proceed, bytes, stat)
      if not ok then
        reply('Neovim could not display the verified target in the originating split')
      elseif failure then
        reply(failure)
      end
    end)
  end

  verify(function(bytes)
    local target, problem = checked_target(request.path, bytes)
    if problem then
      return problem
    end
    target = target or vim.fn.bufadd(request.path)
    vim.fn.bufload(target)
    if not buffer_matches(target, request.path) then
      return unavailable
    end
    problem = verify_buffer(target, bytes)
    if problem then
      return problem
    end
    vim.bo[target].buflisted = true
    if not alive() or not api.nvim_win_is_valid(window) then
      return expired
    end
    api.nvim_set_current_win(window)
    if not alive() or not api.nvim_win_is_valid(window) then
      return expired
    end
    show_target(window, target)

    verify(function(current, stat)
      if api.nvim_win_get_buf(window) ~= target or not buffer_matches(target, request.path) then
        return 'The originating split no longer displays the verified target'
      end
      local _, final_problem = checked_target(request.path, current)
      if final_problem then
        return final_problem
      end
      if request.line > api.nvim_buf_line_count(target) then
        return 'The captured line is outside the target editor buffer'
      end
      if not same_file(stat, uv.fs_lstat(request.path)) then
        return changed
      end
      if not alive() then
        return expired
      end
      api.nvim_win_set_cursor(window, { request.line, 0 })
      api.nvim_win_call(window, function()
        vim.cmd.normal({ bang = true, args = { 'zv' } })
      end)
      reply()
    end)
  end)
end

return M
