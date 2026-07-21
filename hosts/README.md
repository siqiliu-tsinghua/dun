# Reference Plugin Hosts

Example implementations of the Dun Plugin Protocol
([docs/plugin-protocol.md](../docs/plugin-protocol.md)) in three languages.
They exist to prove the protocol is host-neutral and to give plugin authors a
working starting point in whatever language they prefer. None of them is part
of the editor build, the workspace, the CI gates, or the size budget; the only
in-tree host the tests depend on is the Rust fixture host in
`crates/dun-plugin/src/bin/fixture-host.rs`.

The first three serve the `syntax-highlight` role as `user-trusted-external`
hosts: framed stdio (u32 little-endian length + UTF-8 JSON), `hello`/`hello-ack`
handshake, `request` → `response` with `spans` in **character** columns,
clean exit on `shutdown` or EOF.

`python-logfilter/` serves the `log-filter` role and is the first reference host
that exercises the whole capability surface beyond highlighting: its `hello-ack`
contributes a menu subtree and a `Ctrl+L` keybinding leader, each action tagged
with a kind (`scratch`/`execute`/`surface`); it owns an editable scratch window
(the user types a filter substring), an `execute` submit adopts that text as the
pattern, and each command-output stream chunk is filtered to the lines
containing the pattern, shown in the host's surface window. Configure it with
`plugin.logfilter.roles = log-filter`. A Lua log-filter host and a log-filter
conformance path are planned next; the rum version (the only `pure-sandbox`
host) waits on rum-ext.

| Host | Language | Highlighting engine | Notes |
| --- | --- | --- | --- |
| `rust-syntect/` | Rust | [syntect](https://crates.io/crates/syntect) (Sublime grammars) | Standalone cargo project (not a workspace member); reuses `dun-plugin`'s frame/JSON modules as a path dependency. Multi-line constructs highlight correctly within a snapshot because parse state carries across its lines. |
| `python-pygments/` | Python 3 | [Pygments](https://pygments.org/) (`python3-pygments` on Debian) | Single stdlib+Pygments script; Pygments token types map down to dun's five style classes. |
| `lua/` | Lua 5.3+ | Hand-written mini-lexer (keywords/comments/strings/numbers for Lua, Rust, Python) | Zero dependencies, including JSON: the codec is in the script. The smallest full example of the wire format. |

## Configuring a host

```text
plugin.syntect.command = /absolute/path/to/dun-syntect-host
plugin.syntect.trust = user-trusted-external
plugin.syntect.roles = syntax-highlight
```

Build the syntect host with `cargo build --release` inside `hosts/rust-syntect/`
and point `command` at `target/release/dun-syntect-host`.

For the script hosts, note that `dun` launches the command directly (no shell,
no arguments) with a **cleared environment**. A `#!/usr/bin/env python3`
shebang usually still resolves via the OS default path, but interpreters
outside `/usr/bin:/bin` (a venv, a non-default Lua) will not be found. The
robust pattern is a one-line wrapper with absolute paths and pointing
`plugin.<id>.command` at it:

```sh
#!/bin/sh
exec /usr/bin/lua5.4 /absolute/path/to/hosts/lua/dun-lua-host.lua
```

(Debian's `lua5.4` package installs `/usr/bin/lua5.4`; a plain `lua` name is
only present via alternatives.)

## Conformance checking

`check-host.py` (Python stdlib only) drives any host command through
handshake, one highlight request, and shutdown, and validates the wire
behavior dun's client relies on — envelope fields, revision echo, span
bounds in character columns, and the style vocabulary:

```sh
hosts/check-host.py /path/to/host-command
hosts/check-host.py /path/to/host-command --language py --line 'def add(n): return n + 1  # note'
```

Exit code 0 prints an `OK` summary; the first violation aborts with `FAIL`.
Run it with a wrapper script for interpreter-based hosts (it launches the
command with no arguments and an empty environment, matching the editor).

All three hosts pass the checker on Debian (the binding platform) and the two
that run locally on macOS (syntect, Pygments) pass there too.
