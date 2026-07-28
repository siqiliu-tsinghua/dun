#!/usr/bin/env lua
-- Dun Plugin Protocol host in plain Lua (5.3+; no external modules).
--
-- A deliberately dependency-free host that proves the protocol is speakable
-- from a scripting runtime: framed stdio (u32 little-endian length + JSON),
-- a hand-rolled JSON layer, and a small keyword/comment/string/number lexer
-- for a few languages. Configure:
--
--     plugin.lua-highlight.command = /path/to/dun-lua-highlight-host.lua
--     plugin.lua-highlight.trust = user-trusted-external
--     plugin.lua-highlight.roles = syntax-highlight
--
-- Span columns are character offsets (UTF-8 aware via utf8.len).

local HOST_ID = "lua-highlight"
local MAX_SPANS = 4000

-- ---------------------------------------------------------------------------
-- Minimal JSON. The host trusts the editor, so this parser favors clarity
-- over adversarial hardening; the editor-side parser is the audited one.
-- ---------------------------------------------------------------------------

local json = {}

local function skip_ws(text, index)
  local _, stop = text:find("^[ \t\r\n]*", index)
  return stop + 1
end

local parse_value

local function parse_string(text, index)
  assert(text:sub(index, index) == '"', "expected string")
  local out = {}
  index = index + 1
  while true do
    local ch = text:sub(index, index)
    assert(ch ~= "", "unterminated string")
    if ch == '"' then
      return table.concat(out), index + 1
    elseif ch == "\\" then
      local esc = text:sub(index + 1, index + 1)
      if esc == "u" then
        local hex = text:sub(index + 2, index + 5)
        local code = tonumber(hex, 16)
        assert(code, "bad unicode escape")
        -- Surrogate pairs: peek for a following low surrogate.
        if code >= 0xD800 and code < 0xDC00 and text:sub(index + 6, index + 7) == "\\u" then
          local low = tonumber(text:sub(index + 8, index + 11), 16)
          if low and low >= 0xDC00 and low < 0xE000 then
            code = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00)
            index = index + 6
          end
        end
        out[#out + 1] = utf8.char(code)
        index = index + 6
      else
        local map = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" }
        out[#out + 1] = assert(map[esc], "bad escape")
        index = index + 2
      end
    else
      out[#out + 1] = ch
      index = index + 1
    end
  end
end

local function parse_number(text, index)
  local number_text = text:match("^-?%d+%.?%d*[eE]?[+-]?%d*", index)
  local value = tonumber(number_text)
  assert(value, "bad number")
  return value, index + #number_text
end

parse_value = function(text, index)
  index = skip_ws(text, index)
  local ch = text:sub(index, index)
  if ch == '"' then
    return parse_string(text, index)
  elseif ch == "{" then
    local object = {}
    index = skip_ws(text, index + 1)
    if text:sub(index, index) == "}" then
      return object, index + 1
    end
    while true do
      local key
      index = skip_ws(text, index)
      key, index = parse_string(text, index)
      index = skip_ws(text, index)
      assert(text:sub(index, index) == ":", "expected ':'")
      local value
      value, index = parse_value(text, index + 1)
      object[key] = value
      index = skip_ws(text, index)
      local sep = text:sub(index, index)
      if sep == "," then
        index = index + 1
      elseif sep == "}" then
        return object, index + 1
      else
        error("expected ',' or '}'")
      end
    end
  elseif ch == "[" then
    local array = {}
    index = skip_ws(text, index + 1)
    if text:sub(index, index) == "]" then
      return array, index + 1
    end
    while true do
      local value
      value, index = parse_value(text, index)
      array[#array + 1] = value
      index = skip_ws(text, index)
      local sep = text:sub(index, index)
      if sep == "," then
        index = index + 1
      elseif sep == "]" then
        return array, index + 1
      else
        error("expected ',' or ']'")
      end
    end
  elseif text:sub(index, index + 3) == "true" then
    return true, index + 4
  elseif text:sub(index, index + 4) == "false" then
    return false, index + 5
  elseif text:sub(index, index + 3) == "null" then
    return nil, index + 4
  else
    return parse_number(text, index)
  end
end

function json.decode(text)
  local value = parse_value(text, 1)
  return value
end

local function encode_string(value)
  local escaped = value:gsub('[%c"\\]', function(ch)
    local map = { ['"'] = '\\"', ["\\"] = "\\\\", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" }
    return map[ch] or string.format("\\u%04x", ch:byte())
  end)
  return '"' .. escaped .. '"'
end

-- Encodes the host's outbound envelopes: values are strings, integers,
-- ordered arrays, or {key order given by `order`} objects.
local function encode(value, order)
  local kind = type(value)
  if kind == "string" then
    return encode_string(value)
  elseif kind == "number" then
    return string.format("%d", value)
  elseif kind == "table" then
    if order then
      local parts = {}
      for _, key in ipairs(order) do
        local field = value[key]
        if field ~= nil then
          local field_order = type(field) == "table" and field.__order or nil
          parts[#parts + 1] = encode_string(key) .. ":" .. encode(field, field_order)
        end
      end
      return "{" .. table.concat(parts, ",") .. "}"
    end
    local parts = {}
    for _, item in ipairs(value) do
      parts[#parts + 1] = encode(item, type(item) == "table" and item.__order or nil)
    end
    return "[" .. table.concat(parts, ",") .. "]"
  end
  return "null"
end

-- ---------------------------------------------------------------------------
-- Framing
-- ---------------------------------------------------------------------------

local function read_frame()
  local header = io.stdin:read(4)
  if not header or #header < 4 then
    return nil
  end
  local length = string.unpack("<I4", header)
  local payload = io.stdin:read(length)
  if not payload or #payload < length then
    return nil
  end
  return json.decode(payload)
end

local function write_frame(text)
  io.stdout:write(string.pack("<I4", #text), text)
  io.stdout:flush()
end

local ENVELOPE_ORDER = { "v", "kind", "request_id", "plugin_id", "role", "revision", "payload" }

local function send(kind, request_id, revision, payload, payload_order)
  if payload then
    payload.__order = payload_order
  end
  local message = {
    __order = ENVELOPE_ORDER,
    v = 0,
    kind = kind,
    request_id = request_id,
    plugin_id = HOST_ID,
    role = "syntax-highlight",
    revision = revision,
    payload = payload,
  }
  write_frame(encode(message, ENVELOPE_ORDER))
end

-- ---------------------------------------------------------------------------
-- A small lexer: keywords, line comments, strings, numbers.
-- ---------------------------------------------------------------------------

local LANGUAGES = {
  lua = {
    comment = "%-%-",
    keywords = {
      "and", "break", "do", "else", "elseif", "end", "false", "for",
      "function", "goto", "if", "in", "local", "nil", "not", "or",
      "repeat", "return", "then", "true", "until", "while",
    },
  },
  rs = {
    comment = "//",
    keywords = {
      "as", "break", "const", "continue", "crate", "else", "enum", "extern",
      "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
      "mod", "move", "mut", "pub", "ref", "return", "self", "static",
      "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
      "while",
    },
  },
  py = {
    comment = "#",
    keywords = {
      "and", "as", "assert", "async", "await", "break", "class", "continue",
      "def", "del", "elif", "else", "except", "finally", "for", "from",
      "global", "if", "import", "in", "is", "lambda", "None", "not", "or",
      "pass", "raise", "return", "True", "False", "try", "while", "with",
      "yield",
    },
  },
}

local function keyword_set(language)
  local set = {}
  for _, word in ipairs(language.keywords) do
    set[word] = true
  end
  return set
end

local function char_col(line, byte_index)
  return utf8.len(line, 1, byte_index - 1) or (byte_index - 1)
end

local function lex_line(spec, keywords, line, line_number, spans)
  -- Line comment: everything from the marker onward.
  local comment_start = line:find(spec.comment)
  if comment_start then
    spans[#spans + 1] = {
      __order = { "line", "start_col", "end_col", "style" },
      line = line_number,
      start_col = char_col(line, comment_start),
      end_col = utf8.len(line) or #line,
      style = "comment",
    }
    line = line:sub(1, comment_start - 1)
  end
  -- Strings: naive double-quoted ranges.
  for start_byte, stop_byte in line:gmatch('()"[^"]*"()') do
    spans[#spans + 1] = {
      __order = { "line", "start_col", "end_col", "style" },
      line = line_number,
      start_col = char_col(line, start_byte),
      end_col = char_col(line, stop_byte),
      style = "string",
    }
  end
  -- Words: keywords and numbers outside strings (naive but demonstrative).
  for start_byte, word in line:gmatch("()([%w_]+)") do
    local style
    if keywords[word] then
      style = "keyword"
    elseif word:match("^%d+$") then
      style = "number"
    end
    if style then
      spans[#spans + 1] = {
        __order = { "line", "start_col", "end_col", "style" },
        line = line_number,
        start_col = char_col(line, start_byte),
        end_col = char_col(line, start_byte) + (utf8.len(word) or #word),
        style = style,
      }
    end
  end
end

local function highlight(language_name, first_line, lines)
  local spec = LANGUAGES[language_name]
  local spans = {}
  if not spec then
    return spans
  end
  local keywords = keyword_set(spec)
  for offset, line in ipairs(lines) do
    lex_line(spec, keywords, line, first_line + offset - 1, spans)
    if #spans >= MAX_SPANS then
      break
    end
  end
  return spans
end

-- ---------------------------------------------------------------------------
-- Main loop
-- ---------------------------------------------------------------------------

while true do
  local ok, message = pcall(read_frame)
  if not ok or not message then
    os.exit(0)
  end
  local kind = message.kind or ""
  local request_id = message.request_id or 0
  if kind == "hello" then
    send("hello-ack", request_id, nil, {
      host_id = HOST_ID,
      trust = "user-trusted-external",
    }, { "host_id", "trust" })
  elseif kind == "request" then
    local payload = message.payload or {}
    local spans = highlight(payload.language or "", payload.first_line or 0, payload.lines or {})
    send("response", request_id, message.revision, { spans = spans }, { "spans" })
  elseif kind == "cancel-request" then
    -- nothing in flight to cancel; requests are handled synchronously
  elseif kind == "shutdown" then
    os.exit(0)
  else
    send("error", request_id, nil, { message = "unsupported message kind" }, { "message" })
  end
end
