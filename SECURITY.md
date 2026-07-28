# Security Policy

## Reporting a vulnerability

Please report security issues privately through GitHub's **Report a
vulnerability** button on this repository's Security tab, rather than by
opening a public issue. That channel is private to the maintainers until a fix
is published.

Include what you did, what happened, and what you expected. A terminal
transcript or a file that reproduces the behaviour is the most useful thing you
can attach.

## What is in scope

`dun` runs on machines you have already trusted with your files, so the
interesting boundary is what it does with *untrusted input*: file contents,
file names, command output, and plugin responses. Reports in these areas are
especially welcome:

- **Terminal injection.** Any path by which bytes from a file, a file name, a
  command's output, or a plugin reach the terminal without passing the display
  sanitizer — control sequences, OSC strings, bidirectional overrides,
  zero-width or tag characters.
- **File corruption or loss.** Any way to make a save write to the wrong place,
  destroy a file it should have refused to touch, replace a symlink with a
  regular file, or corrupt a buffer opened through the read-only fallback path.
- **Plugin boundary escapes.** Any way for a plugin host to reach editor state
  its role was not granted, to exceed a validated bound, to escalate its trust
  class, or to hang or crash the editor rather than being dropped.
- **Denial of service from ordinary input.** A file, log line, or terminal
  response that makes `dun` hang, exhaust memory, or spin.

[docs/dev/AUDIT.md](docs/dev/AUDIT.md) describes these boundaries in detail and
maps each invariant to the test that carries it. If you find a gap in that map,
that is worth reporting too.

## What is out of scope

- **What a plugin host does outside the protocol.** Hosts declared
  `user-trusted-external` are ordinary programs with ordinary operating-system
  authority; `dun` protects its own state, terminal, and memory from them, and
  says so rather than implying a sandbox. A host reading your files is the
  host's authority, not an escape from `dun`.
- **Terminal emulator behaviour.** Whether a terminal answers an OSC 52
  clipboard read, delivers a modifier key, or renders a glyph is the
  emulator's policy. Real cases are documented in
  [docs/dev/terminal-compatibility-checks.md](docs/dev/terminal-compatibility-checks.md).
- **Configuration you wrote.** Granting a plugin a capability in your config is
  consent; that is what the trust class is for.

## Supported versions

`dun` is pre-1.0. Fixes land on the default branch and in the next release;
there are no maintained back-branches yet.
