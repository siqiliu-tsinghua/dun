# Writing a dun Plugin

A `dun` plugin is a **separate program**. `dun` launches it as a child process
and talks to it over stdin and stdout with framed JSON messages. Your host never
enters the editor's address space, cannot be a shared library, and reaches only
what the protocol exposes.

That means you can write one in whatever language you like — the shipped
reference hosts are Rust, Python, and dependency-free Lua — and that a crash in
your plugin cannot take the editor with it.

This guide is the practical path. [plugin-protocol.md](./plugin-protocol.md) is
the specification: read it when you need the exact rule, read this to get
something working.

- [Pick a role](#pick-a-role)
- [The wire](#the-wire)
- [The lifecycle](#the-lifecycle)
- [A minimal host](#a-minimal-host)
- [Role: syntax-highlight](#role-syntax-highlight)
- [Role: log-filter](#role-log-filter)
- [Contributing a menu and a keybinding](#contributing-a-menu-and-a-keybinding)
- [Installing and configuring](#installing-and-configuring)
- [What the client enforces](#what-the-client-enforces)
- [Testing your host](#testing-your-host)
- [Checklist](#checklist)

## Pick a role

A **role** is a named bundle of capabilities: what your plugin may see, and
which typed channels `dun` will accept output on. It is not a permission your
plugin holds over the system — the protocol has no fields for asking `dun` to
save a file or run a command, so no message can express such a request.

Two roles exist today:

| Role | Receives | May produce |
| --- | --- | --- |
| `syntax-highlight` | a bounded snapshot of buffer text plus a language hint | style spans over that text |
| `log-filter` | command-output stream chunks, and text the user typed into your scratch window | keep/drop verdicts per line, text written into your own window |

A `syntax-highlight` host structurally cannot emit an edit; `dun` runs no
validator that would accept one from it. That is the point of the model: the
narrow role is the safe one, and it is also the easy one to write.

`log-filter` additionally holds the UI-invasive capabilities — contributing a
menu, owning up to two windows, taking scratch input, and executing a submitted
snippet *in your host's own interpreter*. Those require the
`user-trusted-external` trust class and an explicit opt-in in the user's config.

## The wire

Frames on stdin and stdout:

```text
u32 little-endian payload length
UTF-8 JSON payload
```

Three rules that will bite you if you skip them:

- **stdout carries protocol frames and nothing else.** A stray `print()` is a
  malformed frame, and a malformed frame ends the session.
- **stderr is for humans.** `dun` captures it as diagnostics and never parses
  it. Log there freely.
- **Flush after every frame.** Buffered stdout looks exactly like a hung host,
  and a hung host gets killed on timeout.

Every message is an envelope:

```json
{
  "v": 0,
  "kind": "response",
  "request_id": 7,
  "plugin_id": "myhost",
  "revision": 42,
  "payload": { }
}
```

`v` is the protocol version and is currently `0`. Send it on **every** message,
including the `hello-ack` — a handshake without it is rejected before your
payload is looked at.

`kind` is one of `hello`, `hello-ack`, `request`, `response`, `diagnostic`,
`cancel-request`, `error`, `shutdown`. Echo the `request_id` you were given.

A role-specific `response` must also name its `role` — `dun` routes the payload
to that role's validator, and a response that does not say which one is
rejected rather than guessed at.

Echo the `revision` on anything derived from buffer or stream content — `dun`
discards a response whose revision no longer matches, which is how a slow host
cannot paint stale highlighting over edited text.

## The lifecycle

1. `dun` launches your command directly — no shell, minimal environment, only
   stdin/stdout/stderr.
2. `dun` sends `hello`. You reply `hello-ack` with your host id, your trust
   class, and — if you are contributing UI — your menu and keybinding.
3. `dun` sends `request` frames. You reply `response`, or `error` for a failure
   you want reported without dying.
4. `dun` may send `cancel-request`. If you answer synchronously, ignoring it is
   correct.
5. `dun` sends `shutdown`. Exit.

**When you are launched depends on your grant.** A host holding `menu` or
`window` starts eagerly at editor startup, because only its handshake can
advertise the UI it contributes. A highlight-only host launches lazily on its
first job, so an editor session that never opens a source file never pays for
it.

## A minimal host

A complete, working `syntax-highlight` host that marks every line's leading
`#` comment — under sixty lines, no dependencies. Copy it to a file, make it
executable, and check it with `hosts/check-host.py` before wiring it up; this
one passes:

```python
#!/usr/bin/env python3
import json, struct, sys

HOST_ID = "minimal"
PROTOCOL_VERSION = 0

def read_frame(stream):
    header = stream.read(4)
    if len(header) < 4:
        return None
    (length,) = struct.unpack("<I", header)
    payload = stream.read(length)
    return json.loads(payload.decode("utf-8")) if len(payload) == length else None

def write_frame(stream, message):
    data = json.dumps(message, separators=(",", ":")).encode("utf-8")
    stream.write(struct.pack("<I", len(data)))
    stream.write(data)
    stream.flush()

def spans_for(first_line, lines):
    spans = []
    for offset, text in enumerate(lines):
        stripped = text.lstrip()
        if stripped.startswith("#"):
            start = len(text) - len(stripped)
            spans.append({
                "line": first_line + offset,
                "start_col": start,
                "end_col": len(text),
                "style": "comment",
            })
    return spans

def main():
    stdin, stdout = sys.stdin.buffer, sys.stdout.buffer
    while True:
        message = read_frame(stdin)
        if message is None:
            return 0
        kind = message.get("kind", "")
        request_id = message.get("request_id", 0)
        envelope = {"v": PROTOCOL_VERSION,
                    "request_id": request_id, "plugin_id": HOST_ID}
        if kind == "hello":
            write_frame(stdout, {**envelope, "kind": "hello-ack", "payload": {
                "host_id": HOST_ID, "trust": "user-trusted-external"}})
        elif kind == "request":
            payload = message.get("payload") or {}
            write_frame(stdout, {**envelope, "kind": "response",
                "role": "syntax-highlight",
                "revision": message.get("revision"),
                "payload": {"spans": spans_for(
                    payload.get("first_line", 0), payload.get("lines", []))}})
        elif kind == "shutdown":
            return 0

if __name__ == "__main__":
    sys.exit(main())
```

Point `dun` at it, and comments turn the theme's comment color:

```
plugin.minimal.command = /path/to/minimal-host.py
plugin.minimal.trust = user-trusted-external
plugin.minimal.roles = syntax-highlight
```

## Role: syntax-highlight

The request payload gives you a bounded window of the buffer:

| Field | Meaning |
| --- | --- |
| `language` | the file extension, lowercased — that is the whole language hint |
| `first_line` | 0-based line number of `lines[0]` |
| `lines` | the text lines in the snapshot |

Reply with `spans`. Each span is `line`, `start_col`, `end_col`, `style`, where the columns are
**character offsets**, not bytes, and the style is one of `dun`'s
five classes:

```text
keyword   comment   string   number   emphasis
```

`dun` owns that vocabulary deliberately: a plugin cannot invent a color, so a
theme stays coherent and a hostile host cannot repaint the editor. Map your
lexer's token types down to those five — the Pygments host in
`hosts/python-pygments/` is a worked example of that mapping.

Out-of-range coordinates, a span count over the cap, or an unknown style name
means the response is rejected as a whole. Highlighting is best-effort by
design: dropping a bad response costs a repaint, not correctness.

## Role: log-filter

This is the UI-invasive role, and the shipped hosts in `hosts/python-logfilter/`
and `hosts/lua-logfilter/` are the reference. The loop it implements:

1. The user runs a command with `Ctrl+X,O`. Its output becomes a stream.
2. `dun` sends you the stream in bounded chunks — each carrying `chunk`, its
   `index`, and whether it is `final`. Long output arrives as several chunks in
   order; a 2000-line log is not one giant frame.
3. For each line you return a keep/drop verdict — one boolean per line, in
   order.
4. Kept lines accumulate into **your own window**, a plugin-owned surface you
   write validated, sanitized text into.

The pattern itself comes from the user through a **scratch window**: a window
backed by a real `dun` editable buffer, which the user edits with the editor's
own engine — no keystroke routing, no input handling on your side. An
`execute` action submits the whole buffer text to you as one blob. What you do
with that snippet runs in *your* interpreter, never in `dun`.

## Contributing a menu and a keybinding

Declare them in your `hello-ack` payload. They are honored only if your role
holds the `menu` and `keybinding` capabilities:

```json
{
  "host_id": "logfilter",
  "trust": "user-trusted-external",
  "menu": {
    "top_label": {"en_US": "Log Filter", "zh-CN": "日志过滤"},
    "top_mnemonic": "L",
    "items": [
      {"label": {"en_US": "Edit Pattern"},  "mnemonic": "E",
       "action_id": "edit",   "kind": "scratch"},
      {"label": {"en_US": "Apply Pattern"}, "mnemonic": "A",
       "action_id": "apply",  "kind": "execute"},
      {"label": {"en_US": "Show Status"},   "mnemonic": "S",
       "action_id": "status", "kind": "surface"}
    ]
  }
}
```

`kind` tells `dun` what invoking the entry does: `scratch` opens your editable
scratch window, `execute` submits it, `surface` opens or reuses your read-only
surface.

**Declare your mnemonics.** A top-level `top_mnemonic` is optional — omit it and
`dun` derives the first ASCII letter of the `en_US` label — but a dropdown entry
has *no* derivation at all. An item without `mnemonic` is reachable only by
arrows, `Enter`, and the mouse. This is not a detail: a real-terminal pass found
plugin menus keyboard-unreachable in every translated language because a
derivation rule that works in English does not survive translation. Mnemonics
are language-independent, like `dun`'s own, and they are yours to declare.

A mnemonic is one ASCII graphic character. Parentheses are the only exclusion,
because labels render as `label (M)` and the matcher reads the last
parenthesised group.

**Your keybinding lives under `Ctrl+T`.** That leader is reserved for every
plugin, so a plugin can never shadow an editor key. Declare the second strokes;
`dun` composes them. If the user has bound `Ctrl+T` themselves, plugin chords
are disabled — theirs wins.

## Installing and configuring

Installing a plugin is unpacking a folder. Uninstalling it is deleting that
folder. There is no registry, no manifest database, and no install step that
writes into the editor.

**A plugin's own settings live with the plugin.** If your host needs a pattern
file, a theme map, or an API endpoint, read it from your own directory or your
own config file. What goes in `dun`'s config is only how to launch you and what
you are trusted with:

```
plugin.<id>.command = /path/to/host             # required
plugin.<id>.trust = user-trusted-external       # required
plugin.<id>.roles = syntax-highlight, log-filter
plugin.<id>.timeout_ms = 2000
plugin.<id>.max_frame_bytes = 256 KiB
```

The trust class is the grant gate. `user-trusted-external` means "an external
program that speaks the protocol and still has whatever authority the OS gives
it" — the user's config entry is the consent, and it is required for anything
UI-invasive. `pure-sandbox` is reserved for a runtime that provably cannot touch
files, processes, network, or terminal; nothing ships in that class yet.
`unsupported-unsafe` is rejected.

Declaring a trust class in your `hello-ack` that exceeds the configured one
gets you rejected, so you cannot promote yourself.

## What the client enforces

Design for these rather than against them:

- **Frame caps** on stdin, stdout, and stderr capture, applied before
  allocation. Oversized output kills the session.
- **Timeouts**, per request and per host. A host that stops answering is killed
  and reported, not waited on.
- **Revision guards.** A response whose revision no longer matches the buffer or
  stream is discarded.
- **Validation of every result** before it touches editor state — span ranges,
  span counts, style names, surface sizes, menu shapes, chord syntax.
- **Window limits**: at most two windows per plugin, and you may destroy only
  your own.
- **Death is contained.** A crash, a malformed frame, or a protocol error
  becomes a bounded status message and a status-bar chip; editor state is not
  rolled back or corrupted.

Nothing here sandboxes your host. These controls protect `dun`'s state,
terminal, memory, and responsiveness — not the user's machine from your code.
Say so honestly in your own documentation.

## Testing your host

A language-agnostic conformance check ships with the repository. It drives any
host command through handshake, one `syntax-highlight` request, and shutdown,
validating the wire behavior the client relies on:

```sh
hosts/check-host.py /path/to/your-host
hosts/check-host.py /path/to/your-host --language rs --line 'fn main() {}'
```

Exit code `0` means conformant; anything else prints the first violation. It
uses only the Python standard library.

Then run it for real. Configure it, start `dun`, and use:

- `F6` — Config Diagnostics: what was loaded, from where, and any launch error.
- `plugin` at the command prompt — every host's state.
- `plugin load <id>` / `plugin unload <id>` — restart a host after you change
  it, without restarting the editor.
- `plugins.status_bar = true` — a status-bar chip per host, including its error
  state.

For a UI-contributing host, look at the result on a real terminal rather than
trusting the code. `acceptance/sweep-logfilter.sh` captures the log-filter
host's full layout — injected menu, its two windows, and an editor split — as
text grids in a detached `tmux`, with no GUI terminal involved.

## Checklist

Before you publish a host:

- [ ] stdout carries only frames; every frame is flushed.
- [ ] stderr carries your logs, and nothing you need parsed.
- [ ] `request_id` is echoed; `revision` is echoed on content-derived replies.
- [ ] Unknown message kinds get an `error` reply, not a crash.
- [ ] `shutdown` exits promptly.
- [ ] Every dropdown entry declares a `mnemonic`.
- [ ] Labels are provided for the locales you support; `en_US` is the fallback.
- [ ] Your own settings live in your own folder, not in `dun`'s config.
- [ ] `hosts/check-host.py` passes.
- [ ] Your README states plainly what authority your host has outside the
      protocol.
