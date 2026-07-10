#!/usr/bin/env python3
"""Dun Plugin Protocol host: syntax highlighting via Pygments.

Speaks the framed-stdio protocol from docs/plugin-protocol.md as a
`user-trusted-external` host for the `syntax-highlight` role. Requires the
`pygments` package. Configure in the dun config file:

    plugin.pygments.command = /path/to/dun-pygments-host.py
    plugin.pygments.trust = user-trusted-external
    plugin.pygments.roles = syntax-highlight

Frames are `u32 little-endian length + UTF-8 JSON`. Span columns are
character offsets (Python string indices are exactly that). The style
vocabulary is dun's five classes; Pygments token types map down to them.
"""

import json
import struct
import sys

from pygments.lexers import get_lexer_for_filename
from pygments.token import Token
from pygments.util import ClassNotFound

PROTOCOL_VERSION = 0
HOST_ID = "pygments"
MAX_SPANS = 4000

# Most-specific Pygments token prefixes first; dun owns the vocabulary.
STYLE_MAP = [
    (Token.Comment, "comment"),
    (Token.Literal.String, "string"),
    (Token.Literal.Number, "number"),
    (Token.Keyword, "keyword"),
    (Token.Operator.Word, "keyword"),
    (Token.Name.Function, "emphasis"),
    (Token.Name.Class, "emphasis"),
    (Token.Name.Builtin, "keyword"),
]


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


def envelope(kind, request_id, plugin_id, payload, revision=None, role=None):
    message = {
        "v": PROTOCOL_VERSION,
        "kind": kind,
        "request_id": request_id,
        "plugin_id": plugin_id,
        "payload": payload,
    }
    if revision is not None:
        message["revision"] = revision
    if role is not None:
        message["role"] = role
    return message


def style_for_token(token_type):
    for prefix, style in STYLE_MAP:
        if token_type in prefix:
            return style
    return None


def highlight(language, first_line, lines):
    """Tokenize the visible window and emit dun spans (char columns)."""
    text = "\n".join(lines)
    try:
        lexer = get_lexer_for_filename(f"snippet.{language or 'txt'}")
    except ClassNotFound:
        return []

    spans = []
    line_index = 0
    column = 0
    for _, token_type, value in lexer.get_tokens_unprocessed(text):
        pieces = value.split("\n")
        for piece_index, piece in enumerate(pieces):
            if piece_index > 0:
                line_index += 1
                column = 0
            if not piece:
                continue
            style = style_for_token(token_type)
            if style is not None and line_index < len(lines):
                spans.append(
                    {
                        "line": first_line + line_index,
                        "start_col": column,
                        "end_col": column + len(piece),
                        "style": style,
                    }
                )
            column += len(piece)
        if len(spans) >= MAX_SPANS:
            break
    return spans[:MAX_SPANS]


def main():
    stdin = sys.stdin.buffer
    stdout = sys.stdout.buffer
    while True:
        message = read_frame(stdin)
        if message is None:
            return 0
        kind = message.get("kind", "")
        request_id = message.get("request_id", 0)
        plugin_id = message.get("plugin_id", HOST_ID)
        if kind == "hello":
            write_frame(
                stdout,
                envelope(
                    "hello-ack",
                    request_id,
                    plugin_id,
                    {"host_id": HOST_ID, "trust": "user-trusted-external"},
                ),
            )
        elif kind == "request":
            payload = message.get("payload") or {}
            spans = highlight(
                payload.get("language", ""),
                payload.get("first_line", 0),
                payload.get("lines", []),
            )
            write_frame(
                stdout,
                envelope(
                    "response",
                    request_id,
                    plugin_id,
                    {"spans": spans},
                    revision=message.get("revision"),
                    role="syntax-highlight",
                ),
            )
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
