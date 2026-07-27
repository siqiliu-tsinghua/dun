#!/usr/bin/env python3
"""Dun Plugin Protocol host: a log filter.

Speaks the framed-stdio protocol from docs/plugin-protocol.md as a
`user-trusted-external` host for the `log-filter` role. It is the first
reference host that exercises the whole capability surface beyond
syntax-highlight: it contributes a menu and a keybinding leader, owns an
editable scratch window and a surface window, and filters the command-output
stream.

Behavior: the host keeps one filter pattern (a plain substring). "Edit
Pattern" opens the scratch window; the user types a substring there. "Apply
Pattern" submits the scratch text (execute), which becomes the pattern. Each
command-output stream chunk is then filtered to the lines that contain the
pattern (an empty pattern keeps every line), and the kept lines appear in the
host's surface window. "Show Status" prints the current pattern.

Requires only the Python standard library. Configure in the dun config file:

    plugin.logfilter.command = /path/to/dun-python-logfilter-host.py
    plugin.logfilter.trust = user-trusted-external
    plugin.logfilter.roles = log-filter

Frames are `u32 little-endian length + UTF-8 JSON`. dun launches the command
directly (no shell, cleared environment), so use an absolute-path wrapper if
your interpreter is outside /usr/bin:/bin (see hosts/README.md).
"""

import json
import struct
import sys

PROTOCOL_VERSION = 0
HOST_ID = "logfilter"


def read_frame(stream):
    header = stream.read(4)
    if len(header) < 4:
        return None
    (length,) = struct.unpack("<I", header)
    payload = stream.read(length)
    if len(payload) < length:
        return None
    return json.loads(payload.decode("utf-8"))


def write_frame(stream, message):
    payload = json.dumps(message, separators=(",", ":")).encode("utf-8")
    stream.write(struct.pack("<I", len(payload)))
    stream.write(payload)
    stream.flush()


def envelope(kind, request_id, plugin_id, payload):
    return {
        "v": PROTOCOL_VERSION,
        "kind": kind,
        "request_id": request_id,
        "plugin_id": plugin_id,
        "payload": payload,
    }


def hello_payload():
    """HelloAck carries the trust class plus the menu and keybinding the host
    contributes. Each menu item / chord declares an action `kind` so dun knows
    whether it opens a scratch window, executes, or opens a surface."""
    return {
        "host_id": HOST_ID,
        "trust": "user-trusted-external",
        "menu": {
            # No top_mnemonic: dun derives `L` from the en_US label's first
            # letter. Entry mnemonics have no such derivation and must be
            # declared, or the entries are reachable only by arrows, Enter and
            # the mouse. They are language-independent, like dun's own.
            "top_label": {"en_US": "Log Filter", "zh-CN": "日志过滤"},
            "items": [
                {
                    "label": {"en_US": "Edit Pattern"},
                    "mnemonic": "E",
                    "action_id": "edit",
                    "kind": "scratch",
                },
                {
                    "label": {"en_US": "Apply Pattern"},
                    "mnemonic": "A",
                    "action_id": "apply",
                    "kind": "execute",
                },
                {
                    "label": {"en_US": "Show Status"},
                    "mnemonic": "S",
                    "action_id": "status",
                    "kind": "surface",
                },
            ],
        },
        "keybinding": {
            "leader": "Ctrl+T",
            "chords": [
                {"key": "e", "action_id": "edit", "kind": "scratch"},
                {"key": "a", "action_id": "apply", "kind": "execute"},
                {"key": "s", "action_id": "status", "kind": "surface"},
            ],
        },
    }


def handle_request(payload, pattern):
    """Return (reply_payload, new_pattern) for one request. The payload shape
    disambiguates the capability: `snippet` = execute, `stream_id` = a stream
    chunk, `action_id` alone = a surface action."""
    if "snippet" in payload:  # execute: adopt the submitted text as the pattern
        pattern = payload["snippet"].strip()
        summary = (
            f"Filter pattern set to: {pattern!r}"
            if pattern
            else "Filter cleared — keeping every line"
        )
        return {"lines": [summary]}, pattern

    if "stream_id" in payload:  # stream-read: keep lines containing the pattern
        lines = payload.get("lines", [])
        keep = [pattern == "" or pattern in line for line in lines]
        return {"keep": keep}, pattern

    if "action_id" in payload:  # surface: show current status
        status = (
            f"Current filter: {pattern!r}"
            if pattern
            else "No filter set — keeping every line"
        )
        return {
            "lines": [
                status,
                "",
                "Ctrl+T e  edit pattern   Ctrl+T a  apply   Ctrl+T s  status",
            ]
        }, pattern

    return {"message": "unrecognized request payload"}, pattern


def main():
    stdin = sys.stdin.buffer
    stdout = sys.stdout.buffer
    pattern = ""
    while True:
        message = read_frame(stdin)
        if message is None:
            return 0
        kind = message.get("kind", "")
        request_id = message.get("request_id", 0)
        plugin_id = message.get("plugin_id", HOST_ID)
        if kind == "hello":
            write_frame(stdout, envelope("hello-ack", request_id, plugin_id, hello_payload()))
        elif kind == "request":
            payload = message.get("payload") or {}
            reply, pattern = handle_request(payload, pattern)
            reply_kind = "error" if "message" in reply else "response"
            write_frame(stdout, envelope(reply_kind, request_id, plugin_id, reply))
        elif kind == "cancel-request":
            continue
        elif kind == "shutdown":
            return 0
        else:
            write_frame(
                stdout,
                envelope(
                    "error",
                    request_id,
                    plugin_id,
                    {"message": f"unsupported message kind {kind!r}"},
                ),
            )


if __name__ == "__main__":
    sys.exit(main())
