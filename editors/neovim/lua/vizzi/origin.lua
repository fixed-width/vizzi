local uv = vim.uv
local navigation = require('vizzi.navigation')
local M = {}

local protocol = 'vizzi-origin'
local frame_limit = 64 * 1024
local source_limit = 16 * 1024 * 1024

local function hex(value)
  return type(value) == 'string' and #value == 64 and value:match('^[0-9a-f]+$') ~= nil
end

local function credential()
  return (assert(uv.random(32)):gsub('.', function(byte)
    return ('%02x'):format(byte:byte())
  end))
end

local function integer(value, min, max)
  return type(value) == 'number' and value >= min and value <= max and value % 1 == 0
end

local function now_ms()
  local seconds, micros = uv.gettimeofday()
  return seconds * 1000 + math.floor(micros / 1000)
end

local function valid_location(request)
  return type(request.path) == 'string'
    and #request.path <= 4096
    and request.path:sub(1, 1) == '/'
    and not request.path:find('\0', 1, true)
    and integer(request.line, 1, 2 ^ 53 - 1)
    and integer(request.expires_at_ms, 0, 2 ^ 53 - 1)
    and type(request.source) == 'table'
    and integer(request.source.bytes, 0, source_limit)
    and hex(request.source.sha256)
end

local function serve(origin, server, err)
  if err or server:is_closing() then
    return
  end
  local socket = assert(uv.new_tcp())
  if not server:accept(socket) or origin.count >= 4 then
    socket:close()
    return
  end
  local timer = assert(uv.new_timer())
  local function disconnect()
    if socket:is_closing() then
      return
    end
    origin.clients[socket] = nil
    origin.count = origin.count - 1
    if origin.active == socket then
      origin.active = nil
    end
    timer:close()
    socket:close()
  end
  origin.clients[socket] = disconnect
  origin.count = origin.count + 1
  timer:start(12000, 0, disconnect)

  local function dispatch(frame)
    if socket:is_closing() then
      return
    end
    local ok, request = pcall(vim.json.decode, frame)
    if not ok or type(request) ~= 'table' or request.protocol ~= protocol
      or request.version ~= 1 or request.root ~= origin.root or request.session ~= origin.session
      or request.token ~= origin.token or not hex(request.request) then
      disconnect()
      return
    end
    local function reply(problem)
      if socket:is_closing() then
        return
      end
      local response = {
        protocol = protocol,
        version = 1,
        session = origin.session,
        request = request.request,
        status = problem and 'error' or 'opened',
        message = problem,
      }
      if not socket:write(vim.json.encode(response) .. '\n', disconnect) then
        disconnect()
      end
    end
    if not valid_location(request) then
      reply('Invalid verified editor location')
    elseif now_ms() >= request.expires_at_ms then
      reply('The originating editor navigation deadline expired')
    elseif origin.active then
      reply('Originating editor navigation is busy; retry')
    else
      origin.active = socket
      local deadline = uv.hrtime() + 8e9
      navigation.open(origin.window, request, function()
        return not socket:is_closing() and uv.hrtime() < deadline and now_ms() < request.expires_at_ms
      end, reply)
    end
  end

  local chunks, size = {}, 0
  local started = socket:read_start(function(read_error, data)
    if read_error or not data or not chunks or size + #data > frame_limit then
      disconnect()
      return
    end
    local newline = data:find('\n', 1, true)
    if newline and newline ~= #data then
      disconnect()
      return
    end
    size = size + #data
    chunks[#chunks + 1] = data
    if newline then
      local frame = table.concat(chunks)
      chunks = nil
      vim.schedule(function()
        dispatch(frame)
      end)
    end
  end)
  if not started then
    disconnect()
  end
end

function M.start(root, window)
  local origin = {
    root = root,
    window = window,
    session = credential(),
    token = credential(),
    clients = {},
    count = 0,
  }
  local listener = assert(uv.new_tcp())
  local directory
  local file
  local function close()
    if not listener:is_closing() then
      listener:close()
      for _, disconnect in pairs(origin.clients) do
        disconnect()
      end
      if file then
        uv.fs_unlink(file)
      end
      if directory then
        uv.fs_rmdir(directory)
      end
    end
  end

  local ok, failure = pcall(function()
    local runtime = assert(uv.fs_realpath(vim.fn.stdpath('run')))
    assert(root ~= '/' and runtime ~= root and runtime:sub(1, #root + 1) ~= root .. '/',
      'Vizzi runtime storage must be outside captured source')
    directory = assert(uv.fs_mkdtemp(vim.fs.joinpath(runtime, 'vizzi-XXXXXX')))
    assert(listener:bind('127.0.0.1', 0))
    assert(listener:listen(4, function(err)
      serve(origin, listener, err)
    end))
    local descriptor = vim.json.encode({
      protocol = protocol,
      version = 1,
      root = root,
      editor = 'Neovim',
      session = origin.session,
      port = assert(listener:getsockname()).port,
      token = origin.token,
    })
    assert(#descriptor <= 4096, 'Vizzi origin descriptor exceeds the protocol allowance')
    file = vim.fs.joinpath(directory, 'origin.json')
    local fd = assert(uv.fs_open(file, 'wx', tonumber('600', 8)))
    local written, write_error = uv.fs_write(fd, descriptor, 0)
    uv.fs_close(fd)
    assert(written == #descriptor, write_error or 'Incomplete Vizzi origin descriptor')
  end)
  if not ok then
    close()
    error(failure, 0)
  end
  return { file = file, close = close }
end

return M
