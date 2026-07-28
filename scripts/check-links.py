#!/usr/bin/env python3
"""Check that every local link in the repository's Markdown files resolves.

Walks the git-tracked Markdown files, extracts inline links and bare
repository-relative path mentions, and reports the ones whose target does not
exist on disk. External links (http/https/mailto) and pure anchors are not
followed; anchors on local files are checked only for the file part.

Exit status is 0 when every local target resolves and 1 otherwise, so this can
gate a documentation move.

    scripts/check-links.py            # check the whole tree
    scripts/check-links.py --quiet    # only print the summary line
"""

import argparse
import os
import re
import subprocess
import sys

# [text](target) and [text]: target reference definitions.
INLINE_LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
REF_LINK = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)", re.MULTILINE)

# Bare mentions of repository paths in prose or code spans, e.g. `docs/i18n.md`
# or scripts/release-build.sh. Only paths ending in a known documentation or
# script suffix are considered, to keep the false-positive rate at zero.
BARE_PATH = re.compile(
    r"(?<![\w./-])((?:docs|scripts|crates|hosts|acceptance|vm-test|i18n)/[\w./-]+"
    r"\.(?:md|py|sh|rs|toml|conf|lua))(?![\w/-])"
)

# Source-tree mentions are checked only under --strict. The append-only
# PROGRESS log and the codex briefs describe the tree as it was on the day
# they were written, so a mention of a since-split module is a historical
# fact, not a broken reference. Documentation and tooling paths are always
# checked: those are navigation, and a move must not break them.
HISTORICAL_PREFIXES = ("crates/", "i18n/")

SKIP_PREFIXES = ("http://", "https://", "mailto:", "#")


def tracked_markdown(root):
    out = subprocess.run(
        ["git", "-C", root, "ls-files", "*.md"],
        check=True,
        capture_output=True,
        text=True,
    )
    return [line for line in out.stdout.splitlines() if line]


def targets_in(text):
    """Yield (target, kind) pairs worth resolving."""
    for match in INLINE_LINK.finditer(text):
        yield match.group(1), "link"
    for match in REF_LINK.finditer(text):
        yield match.group(1), "link"
    for match in BARE_PATH.finditer(text):
        yield match.group(1), "mention"


def check(root, quiet=False, strict=False):
    broken = []
    checked = 0
    for rel in tracked_markdown(root):
        path = os.path.join(root, rel)
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        base = os.path.dirname(rel)
        for target, kind in targets_in(text):
            if target.startswith(SKIP_PREFIXES):
                continue
            if (
                kind == "mention"
                and not strict
                and target.startswith(HISTORICAL_PREFIXES)
            ):
                continue
            file_part = target.split("#", 1)[0]
            if not file_part:
                continue
            # Links resolve relative to the file; bare mentions are written as
            # repository-relative paths.
            start = base if kind == "link" else ""
            resolved = os.path.normpath(os.path.join(root, start, file_part))
            checked += 1
            if not os.path.exists(resolved):
                broken.append((rel, target, kind))

    if not quiet:
        for rel, target, kind in broken:
            print(f"{rel}: broken {kind} -> {target}")
    print(
        f"checked {checked} local targets in "
        f"{len(tracked_markdown(root))} markdown files, {len(broken)} broken"
    )
    return 1 if broken else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quiet", action="store_true")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="also check bare mentions of source paths (crates/, i18n/), "
        "which historical documents may legitimately name after a split",
    )
    args = parser.parse_args()
    root = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return check(root, quiet=args.quiet, strict=args.strict)


if __name__ == "__main__":
    sys.exit(main())
