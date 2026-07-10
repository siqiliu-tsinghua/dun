#!/usr/bin/env python3
"""Language-agnostic conformance check for Dun Plugin Protocol hosts.

Drives any host command through handshake, one syntax-highlight request,
and shutdown, validating the wire behavior dun's client relies on. Uses
only the Python standard library.

    hosts/check-host.py <host-command> [--language rs] [--line 'fn main() {}']

Exit code 0 = conformant; nonzero prints the first violation.
"""

import argparse
import json
import struct
import subprocess
import sys

TIMEOUT_SECONDS = 10
ALLOWED_STYLES = {"keyword", "comment", "string", "number", "emphasis"}


def frame(message):
    payload = json.dumps(message, separators=(",", ":")).encode("utf-8")
    return struct.pack("<I", len(payload)) + payload


def read_frame(stream):
    header = stream.read(4)
    if len(header) < 4:
        raise SystemExit("FAIL: host closed the stream before replying")
    (length,) = struct.unpack("<I", header)
    if length > 1 << 20:
        raise SystemExit(f"FAIL: host frame of {length} bytes exceeds sanity cap")
    payload = stream.read(length)
    if len(payload) < length:
        raise SystemExit("FAIL: host truncated a frame")
    try:
        return json.loads(payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise SystemExit(f"FAIL: host frame is not UTF-8 JSON: {error}")


def expect(condition, message):
    if not condition:
        raise SystemExit(f"FAIL: {message}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", help="host executable to check")
    parser.add_argument("--language", default="rs")
    parser.add_argument("--line", default="fn main() { return 42; } // done")
    args = parser.parse_args()

    process = subprocess.Popen(
        [args.command],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        env={},
    )
    try:
        process.stdin.write(
            frame(
                {
                    "v": 0,
                    "kind": "hello",
                    "request_id": 0,
                    "plugin_id": "",
                    "payload": {"host": "dun"},
                }
            )
        )
        process.stdin.flush()
        ack = read_frame(process.stdout)
        expect(ack.get("v") == 0, f"hello-ack protocol version {ack.get('v')!r} != 0")
        expect(ack.get("kind") == "hello-ack", f"expected hello-ack, got {ack.get('kind')!r}")
        payload = ack.get("payload") or {}
        expect(bool(payload.get("host_id")), "hello-ack payload lacks host_id")
        expect(
            payload.get("trust") in {"pure-sandbox", "user-trusted-external"},
            f"hello-ack trust {payload.get('trust')!r} is not a known class",
        )

        lines = [args.line, ""]
        process.stdin.write(
            frame(
                {
                    "v": 0,
                    "kind": "request",
                    "request_id": 7,
                    "plugin_id": "check",
                    "role": "syntax-highlight",
                    "revision": 41,
                    "payload": {
                        "language": args.language,
                        "first_line": 10,
                        "lines": lines,
                    },
                }
            )
        )
        process.stdin.flush()
        response = read_frame(process.stdout)
        expect(
            response.get("kind") == "response",
            f"expected response, got {response.get('kind')!r}",
        )
        expect(response.get("request_id") == 7, "response request_id mismatch")
        expect(response.get("revision") == 41, "response must echo the revision")
        expect(
            response.get("role") == "syntax-highlight",
            "response must carry the syntax-highlight role",
        )
        spans = (response.get("payload") or {}).get("spans")
        expect(isinstance(spans, list), "response payload lacks a span list")
        expect(len(spans) > 0, "host produced no spans for a keyword-bearing line")
        for span in spans:
            line = span.get("line")
            expect(
                isinstance(line, int) and 10 <= line < 10 + len(lines),
                f"span line {line!r} outside the snapshot window",
            )
            start = span.get("start_col")
            end = span.get("end_col")
            line_length = len(lines[line - 10])
            expect(
                isinstance(start, int) and isinstance(end, int) and 0 <= start < end <= line_length,
                f"span columns {start!r}..{end!r} invalid for a {line_length}-char line",
            )
            expect(
                span.get("style") in ALLOWED_STYLES,
                f"span style {span.get('style')!r} not in the dun vocabulary",
            )

        process.stdin.write(
            frame(
                {
                    "v": 0,
                    "kind": "shutdown",
                    "request_id": 0,
                    "plugin_id": "",
                    "payload": None,
                }
            )
        )
        process.stdin.flush()
        code = process.wait(timeout=TIMEOUT_SECONDS)
        expect(code == 0, f"host exited {code} after shutdown")
    finally:
        if process.poll() is None:
            process.kill()

    styles = sorted({span["style"] for span in spans})
    print(f"OK: {len(spans)} span(s), styles {styles}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
