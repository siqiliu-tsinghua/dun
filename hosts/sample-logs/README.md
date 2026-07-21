# Sample Logs for the Log-Filter Hosts

Synthetic log datasets for exercising the log-filter reference hosts
(`../python-logfilter`, `../lua-logfilter`) and for demonstrating a defensive
log-analysis workflow in the editor. Not part of the editor build, the
workspace, the CI gates, or the size budget.

**All data is synthetic.** Every IP address, hostname, username, and request
is an illustrative value produced by `generate.py`; none corresponds to a real
host, person, or event. These are the logs a *target* accumulates — used here
to study and defend against scanning, never as an attack tool. Regenerate with
`generate.py`, or scale up with `generate.py --scale 30` (≈ a busy month). A
fixed seed keeps the committed files stable.

| File | Format | Filter / analysis it demonstrates |
| --- | --- | --- |
| `ssh-bruteforce.log` | syslog `sshd` auth | attacker IP distribution, diurnal scan peaks, tried-username dictionary |
| `mcp-probes.log` | MCP-server access log | triage legit MCP traffic vs path scanners vs exploit probes → fail2ban rules |
| `app-levels.log` | app log with levels | filter `ERROR` / `WARN` — the canonical log-filter case |
| `access.log` | nginx/apache combined | filter `404`, `POST`, a path, or an IP |
| `app.jsonl` | JSON lines | substring-filter structured logs (`"level":"error"`) |
| `stacktrace.log` | app log + multi-line traces | wide / multi-line filtering (`Exception`, `at com.`) |

## Scenario 1 — SSH exposure (`ssh-bruteforce.log`)

A test box left with its SSH port open and **no fail2ban**, running for about a
month, accumulates a large record of port scans and brute-force login attempts.
The dataset models that: brute-force *sessions* (one source IP hammering a
sequence of usernames a few seconds apart), a weighted source pool so a handful
of prolific botnet-style networks dominate the long tail, a diurnal weighting
so scanning peaks in the late-UTC hours, and a few genuine `Accepted publickey`
operator logins mixed in for signal-vs-noise contrast.

Questions it lets you explore (in the editor, run the command and filter the
output; or with the shell directly):

```sh
# Which sources are most persistent? (attacker IP distribution)
grep 'Failed password' ssh-bruteforce.log \
  | grep -oE 'from [0-9.]+' | awk '{print $2}' | sort | uniq -c | sort -rn | head

# When do scans peak? (working-hours / diurnal signature, UTC hour)
grep -oE '^[A-Za-z]+ +[0-9]+ [0-9]+' ssh-bruteforce.log \
  | awk '{print $3}' | sort | uniq -c

# What usernames are tried? (the botnet dictionary)
grep -oE 'user [a-z]+' ssh-bruteforce.log | sort | uniq -c | sort -rn | head
```

In the log-filter host: run `cat .../ssh-bruteforce.log`, then set the pattern
to a single prolific IP to isolate that attacker's whole session, or to
`Accepted` to pull the real logins out of the noise.

## Scenario 2 — MCP server sniffing (`mcp-probes.log`)

A server exposing an MCP endpoint faces the same background sniffing as any web
service. The dataset interleaves **legitimate MCP traffic** (JSON-RPC
`initialize` / `tools/list` / `tools/call` … over `POST /mcp`, from a few known
clients) with scanner sessions probing for **secrets** (`/.env`, `/.git/config`,
`/.aws/credentials`), **CMS/admin** panels (`/wp-login.php`, `/phpmyadmin/`),
and **exploit** paths (path traversal, `think\app` RCE, a log4shell string
smuggled in the User-Agent).

The triage that suggests fail2ban rules:

```sh
# Sources hammering non-existent paths (404 storms) — prime ban candidates
grep '" 404 ' mcp-probes.log | awk '{print $2}' | sort | uniq -c | sort -rn | head

# Anyone probing for secrets — a high-confidence ban signal
grep -E '/\.env|/\.git|/\.aws|credentials' mcp-probes.log | awk '{print $2}' | sort -u

# Scanner user-agents (a UA-based jail)
grep -oE '"(zgrab|masscan|Nuclei|python-requests|Go-http-client)[^"]*"' mcp-probes.log \
  | sort | uniq -c | sort -rn
```

Filtering these classes apart — legit `/mcp` vs `404` scanners vs the secrets
probes — is what a `log-filter` plugin is for: each class becomes a substring
(or, once patterns land, a regex) that a fail2ban `failregex` can mirror.

## Regenerating

```sh
hosts/sample-logs/generate.py            # the committed set (seed-stable)
hosts/sample-logs/generate.py --scale 30 # a month-sized volume
```
