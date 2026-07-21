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
service. The log is written as `<ts> IP - "METHOD path HTTP/1.1" status size
"ua"` — the `IP - "…"` shape a fail2ban `failregex` anchors on via `<HOST>` —
and interleaves:

- **legitimate traffic** on the allow-listed endpoints: the bare `/`, `/mcp`
  (JSON-RPC), the OAuth dance (`/oauth/authorize`, `/oauth/token`), and
  `/.well-known/oauth-*` discovery, from a few known clients. Crucially, *some
  of it legitimately returns 4xx* — a failed `/oauth/token` 401, an
  unauthenticated `/mcp` 401, a missing `/favicon.ico` 404;
- **scanner sessions** probing everything else — secrets (`/.env`,
  `/.git/config`, `/.aws/credentials`), CMS/admin panels, exploit paths
  (traversal, `think\app` RCE, a log4shell string in the User-Agent) —
  returning 401/403/404.

That mix is the whole point: a naive "ban every IP that gets a 4xx" bans your
own clients (the favicon 404, the unauthenticated `/mcp`). The rule has to
**allow-list the real endpoints** and ban 4xx only elsewhere — which is exactly
the real failregex this dataset was built to reproduce:

```
failregex = ^.*\b<HOST>(?::\d+)?\s+-\s+"[A-Z]+\s+/(?!(?:$|mcp(?:[/? ]|$)|oauth[/? ]|\.well-known[/? ]|favicon\.ico(?:[? ]|$)))\S+\s+HTTP/\d(?:\.\d+)?"\s+(?:401|403|404)\b
```

On the committed set that failregex matches **499 lines / 89 distinct IPs** (the
scanners) and **spares 23 allow-listed 4xx** (favicon 404, unauthenticated
`/mcp`, failed `/oauth/token`) — the lines a naive 4xx rule would wrongly ban.
Verify with the exact regex (`<HOST>` → an IP group):

```sh
python3 - <<'EOF'
import re
rx = re.compile(r'^.*\b(?P<h>(?:\d{1,3}\.){3}\d{1,3})(?::\d+)?\s+-\s+"[A-Z]+\s+/'
                r'(?!(?:$|mcp(?:[/? ]|$)|oauth[/? ]|\.well-known[/? ]|favicon\.ico(?:[? ]|$)))'
                r'\S+\s+HTTP/\d(?:\.\d+)?"\s+(?:401|403|404)\b')
ban = {rx.search(l).group("h") for l in open("mcp-probes.log") if rx.search(l)}
print(len(ban), "IPs to ban")
EOF
```

Triaging those classes apart — legit `/mcp` vs the secrets probes vs the CMS
scanners — is what a `log-filter` plugin is for: each class is a substring (or,
once patterns land, a regex) that a `failregex` like the one above mirrors.

## Regenerating

```sh
hosts/sample-logs/generate.py            # the committed set (seed-stable)
hosts/sample-logs/generate.py --scale 30 # a month-sized volume
```
