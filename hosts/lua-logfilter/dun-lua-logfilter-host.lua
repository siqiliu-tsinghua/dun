#!/usr/bin/env lua
-- Dun Plugin Protocol host in plain Lua (5.3+; no external modules): a log
-- filter.
--
-- The dependency-free counterpart of hosts/python-logfilter, and the smallest
-- full example of a host that exercises the whole capability surface beyond
-- syntax-highlight: framed stdio (u32 little-endian length + JSON) with a
-- hand-rolled JSON layer, a `hello-ack` that contributes a menu subtree and a
-- Ctrl+T keybinding leader (each action tagged scratch/execute/surface), and
-- the log-filter request handling. Configure:
--
--     plugin.logfilter.command = /path/to/dun-lua-logfilter-host.lua
--     plugin.logfilter.trust = user-trusted-external
--     plugin.logfilter.roles = log-filter
--
-- Behavior mirrors the Python host: one filter pattern (a plain substring).
-- "Edit Pattern" opens the scratch window; "Apply Pattern" (execute) adopts
-- the scratch text as the pattern; each command-output stream chunk is
-- filtered to the lines containing the pattern (empty pattern keeps every
-- line), shown in the surface window; "Show Status" prints the pattern.

local HOST_ID = "logfilter"

-- ---------------------------------------------------------------------------
-- Minimal JSON. The host trusts the editor, so the parser favors clarity over
-- adversarial hardening; the editor-side parser is the audited one.
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

-- Encodes the host's outbound envelopes: strings, integers, booleans, ordered
-- `{__order=...}` objects, or plain arrays. Booleans matter here (the keep
-- verdict is an array of them) where the highlight host never needed them.
local function encode(value, order)
  local kind = type(value)
  if kind == "string" then
    return encode_string(value)
  elseif kind == "number" then
    return string.format("%d", value)
  elseif kind == "boolean" then
    return value and "true" or "false"
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

local ENVELOPE_ORDER = { "v", "kind", "request_id", "plugin_id", "payload" }

-- Sends one envelope. `payload` carries its own `__order` for field ordering;
-- log-filter responses omit `role` (dun sends surface/stream/execute requests
-- with no role and does not check it on the way back).
local function send(kind, request_id, payload)
  local message = {
    __order = ENVELOPE_ORDER,
    v = 0,
    kind = kind,
    request_id = request_id,
    plugin_id = HOST_ID,
    payload = payload,
  }
  write_frame(encode(message, ENVELOPE_ORDER))
end

-- ---------------------------------------------------------------------------
-- Contributions and request handling
-- ---------------------------------------------------------------------------

-- Entry mnemonics must be declared: dun derives one for the top-level label
-- but deliberately not for entries, so an entry without `mnemonic` has no
-- letter shortcut (arrows/Enter/mouse still reach it). Language-independent,
-- like dun's own.
local function menu_item(label, mnemonic, action_id, kind)
  return {
    __order = { "label", "mnemonic", "action_id", "kind" },
    label = { __order = { "en_US" }, en_US = label },
    mnemonic = mnemonic,
    action_id = action_id,
    kind = kind,
  }
end

local function chord(key, action_id, kind)
  return {
    __order = { "key", "action_id", "kind" },
    key = key,
    action_id = action_id,
    kind = kind,
  }
end

local function hello_payload()
  return {
    __order = { "host_id", "trust", "menu", "keybinding" },
    host_id = HOST_ID,
    trust = "user-trusted-external",
    menu = {
      __order = { "top_label", "items" },
      top_label = { __order = { "en_US", "zh-CN" }, en_US = "Log Filter", ["zh-CN"] = "日志过滤" },
      items = {
        menu_item("Edit Pattern", "E", "edit", "scratch"),
        menu_item("Apply Pattern", "A", "apply", "execute"),
        menu_item("Show Status", "S", "status", "surface"),
      },
    },
    keybinding = {
      __order = { "leader", "chords" },
      leader = "Ctrl+T",
      chords = {
        chord("e", "edit", "scratch"),
        chord("a", "apply", "execute"),
        chord("s", "status", "surface"),
      },
    },
  }
end

-- Returns (reply_payload, new_pattern). The payload shape disambiguates the
-- capability: `snippet` = execute, `stream_id` = a stream chunk, `action_id`
-- alone = a surface action.
local function handle_request(payload, pattern)
  if payload.snippet ~= nil then
    pattern = payload.snippet:gsub("^%s+", ""):gsub("%s+$", "")
    local summary
    if pattern == "" then
      summary = "Filter cleared -- keeping every line"
    else
      summary = "Filter pattern set to: " .. pattern
    end
    return { __order = { "lines" }, lines = { summary } }, pattern
  end

  if payload.stream_id ~= nil then
    local keep = {}
    for _, line in ipairs(payload.lines or {}) do
      keep[#keep + 1] = pattern == "" or line:find(pattern, 1, true) ~= nil
    end
    return { __order = { "keep" }, keep = keep }, pattern
  end

  if payload.action_id ~= nil then
    local status
    if pattern == "" then
      status = "No filter set -- keeping every line"
    else
      status = "Current filter: " .. pattern
    end
    return {
      __order = { "lines" },
      lines = { status, "", "Ctrl+T e  edit pattern   Ctrl+T a  apply   Ctrl+T s  status" },
    }, pattern
  end

  return { __order = { "message" }, message = "unrecognized request payload" }, pattern
end

-- ---------------------------------------------------------------------------
-- Main loop
-- ---------------------------------------------------------------------------

local pattern = ""

while true do
  local ok, message = pcall(read_frame)
  if not ok or not message then
    os.exit(0)
  end
  local kind = message.kind or ""
  local request_id = message.request_id or 0
  if kind == "hello" then
    send("hello-ack", request_id, hello_payload())
  elseif kind == "request" then
    local reply
    reply, pattern = handle_request(message.payload or {}, pattern)
    local reply_kind = reply.message ~= nil and "error" or "response"
    send(reply_kind, request_id, reply)
  elseif kind == "cancel-request" then
    -- nothing in flight to cancel; requests are handled synchronously
  elseif kind == "shutdown" then
    os.exit(0)
  else
    send("error", request_id, { __order = { "message" }, message = "unsupported message kind" })
  end
end
