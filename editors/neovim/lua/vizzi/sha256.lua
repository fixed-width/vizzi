local builtin = vim.fn.sha256
local binary_safe, nul_digest = pcall(builtin, '\0')
if binary_safe and nul_digest == '6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d' then
  return builtin
end

local bit = require('bit')
local band, bnot, bxor = bit.band, bit.bnot, bit.bxor
local ror, rshift, tobit = bit.ror, bit.rshift, bit.tobit

-- SHA-256 initial state and round constants from RFC 6234 sections 6.1 and 5.1.
local initial = {
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
}
local constants = {
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
}

local function big_endian(word)
  return string.char(rshift(word, 24), band(rshift(word, 16), 0xff), band(rshift(word, 8), 0xff), band(word, 0xff))
end

local function compress(bytes, offset, state, words)
  for i = 1, 16 do
    local start = offset + (i - 1) * 4
    local first, second, third, fourth = bytes:byte(start, start + 3)
    words[i] = tobit(first * 0x1000000 + second * 0x10000 + third * 0x100 + fourth)
  end
  for i = 17, 64 do
    local earlier, later = words[i - 15], words[i - 2]
    local sigma0 = bxor(ror(earlier, 7), ror(earlier, 18), rshift(earlier, 3))
    local sigma1 = bxor(ror(later, 17), ror(later, 19), rshift(later, 10))
    words[i] = tobit(words[i - 16] + sigma0 + words[i - 7] + sigma1)
  end

  local a, b, c, d, e, f, g, h = unpack(state)
  for i = 1, 64 do
    local sum1 = bxor(ror(e, 6), ror(e, 11), ror(e, 25))
    local choose = bxor(band(e, f), band(bnot(e), g))
    local first = tobit(h + sum1 + choose + constants[i] + words[i])
    local sum0 = bxor(ror(a, 2), ror(a, 13), ror(a, 22))
    local majority = bxor(band(a, b), band(a, c), band(b, c))
    h, g, f, e, d, c, b, a = g, f, e, tobit(d + first), c, b, a, tobit(first + sum0 + majority)
  end
  state[1] = tobit(state[1] + a)
  state[2] = tobit(state[2] + b)
  state[3] = tobit(state[3] + c)
  state[4] = tobit(state[4] + d)
  state[5] = tobit(state[5] + e)
  state[6] = tobit(state[6] + f)
  state[7] = tobit(state[7] + g)
  state[8] = tobit(state[8] + h)
end

return function(bytes)
  if not bytes:find('\0', 1, true) then
    return builtin(bytes)
  end
  local length = #bytes
  local bits = length * 8
  local padded = bytes .. '\128' .. string.rep('\0', (55 - length) % 64)
    .. big_endian(math.floor(bits / 2 ^ 32)) .. big_endian(bits % 2 ^ 32)
  local state, words = { unpack(initial) }, {}
  for offset = 1, #padded, 64 do
    compress(padded, offset, state, words)
  end
  for i, word in ipairs(state) do
    state[i] = bit.tohex(word)
  end
  return table.concat(state)
end
