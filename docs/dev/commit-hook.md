# The commit-msg hook

`scripts/hooks/commit-msg` guards the AI co-author trailer's email address.

## Why it exists

A sibling project once committed a `Co-Authored-By` trailer whose address was a
typo that happened to match a **real person's** address, silently adding that
person to the repository's contributor list. Once pushed, that does not correct
itself — the trailer is part of the commit object, and rewriting history is the
only way out. So it is checked before the commit object exists.

## Install

The hook is tracked, and `.git/hooks/` is not, so a fresh clone needs one
command:

```sh
ln -sf ../../scripts/hooks/commit-msg .git/hooks/commit-msg
```

A symlink rather than a copy, so editing the tracked file takes effect at once
and the two cannot drift apart.

## What it enforces

1. Any `@anthropic.com` address anywhere in the message must be exactly
   `noreply@anthropic.com`. This is the rule that catches the original bug: a
   real employee address has the form `<name>@anthropic.com`, one typo away.
2. A `Co-Authored-By` trailer whose **name** mentions Claude or Anthropic must
   use `<noreply@anthropic.com>` — this catches a wrong domain, which rule 1
   cannot see.
3. A `Co-Authored-By` trailer must be a well-formed `Name <email>`.

Deliberate exception: `git commit --no-verify`.

It does **not** check the model name. `Co-Authored-By: Claude Opus 4
<noreply@anthropic.com>` passes. A stale model name is cosmetic and
self-correcting; a wrong address credits a stranger. Only the second is worth
blocking a commit over, and hard-coding the current model would mean editing
the hook every time it changes.

## Three things it does that the rum original does not

The hook began as a copy of `rum`'s. Testing it against the cases below turned
up three defects, all fixed here and all worth porting back:

- **`git commit -v` false rejection.** That flag appends the diff below a
  scissors line. rum's hook scans it, so committing *any* change to a file that
  contains an `@anthropic.com` address is blocked — on content that is not part
  of the message at all. This version truncates at the scissors line first.
- **Comment lines were skipped entirely.** rum's hook drops every `#` line
  before scanning. Git only strips those when the message goes through an
  **editor**; `git commit -F` and `git commit -m` keep them verbatim — verified,
  a `#` line lands in the commit object. So a bad address on a `#` line reached
  a real commit with the hook reporting clean. Since the hook cannot know which
  cleanup mode is in play, this version **warns** on comment lines instead of
  either ignoring or rejecting them: no false rejections, no silent misses.
- **Indented trailers were rejected as malformed.** rum's selection grep allows
  leading whitespace but its validation regex does not, so the two disagree and
  a perfectly valid indented trailer is refused. Git treats an indented line as
  a *continuation* of the previous trailer rather than a trailer of its own, so
  this version matches trailers at column 0 only. Rule 1 still covers the
  address on such a line.

## Testing a change to the hook

There is no test harness for it; it is a dozen `case` arms and a symlink. Run it
directly against a message file — `scripts/hooks/commit-msg FILE`, exit status
0 accepts — and cover at least: a correct trailer, no trailer, a human
co-author, a real-person `@anthropic.com` address, a typo'd `noreply`, the right
name with a wrong domain, a malformed trailer, an address in prose, two trailers
where only the second is bad, an address on a `#` line, an indented trailer, and
an address in a diff below a scissors line.
