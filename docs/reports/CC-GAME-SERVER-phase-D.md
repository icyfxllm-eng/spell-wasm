# CC-GAME-SERVER — Phase D report (deploy kit F14, host runbook F15)

**Result: the kit and runbook are written, and every check that can run on
this Mac passes. The container, Quadlet, shellcheck, gitleaks and cargo-deny
checks need tools this Mac doesn't have, and run in the new CI workflow
instead. Two items need your decision.**

Branch `game-server-phase-a`, worktree `~/repos/spell-wasm-game-server`.
Date: 2026-10-25. Nothing here has been run on the Rocky box: you run the
runbook (F15 invariant).

## What exists now (`server/deploy/`)

| File | What |
|---|---|
| `Containerfile` | Two stages. The rust:1-bookworm builder builds `cargo build --release --locked -p spell-server`. Runtime is `gcr.io/distroless/cc-debian12:nonroot`: no shell, a non-root user |
| `spellgame.container` | The user Quadlet unit, exactly per F14: image pinned by digest, both ports published on `127.0.0.1` only, `spell-data:/data:Z`, read-only, `NoNewPrivileges`, all capabilities dropped, 1 GB memory, 256 pids, `Restart=always`, `RestartSec=2`, `TimeoutStopSec=960`, no `AutoUpdate` |
| `cloudflared-config.template.yml` | A new tunnel for Rocky. `<HOSTNAME>` → `127.0.0.1:8080`, then `http_status:404`. Placeholders only, and no mention of the admin port |
| `build.sh` | Builds the image locally (D-X4) and prints the digest |
| `deploy.sh <digest>` | Refuses a unit with placeholders, a non-loopback publish or auto-update. Then: drain → point the unit at the digest → restart (SIGTERM finishes the drain) → wait up to 120 s for healthz → print status and readyz |
| `backup.sh` + `spell-backup.{service,timer}` | 04:17 nightly, `Persistent=true`. `/admin/snapshot` → `podman unshare cat` → `restic backup --stdin`, then `forget --keep-daily 14 --keep-weekly 8 --keep-monthly 6 --prune`, then the stamp file. Secrets live in an env file on the host only |
| `restore-drill.sh` | Restores the latest snapshot to a scratch dir and checks `integrity_check`, the schema against live, and row counts. Prints PASS/FAIL |
| `journald-spell.conf` | `MaxRetentionSec=14day` |
| `HOST-RUNBOOK.md` | Steps 0–14 in order, each with its check and its H-id |
| `verify-host.sh` | H0–H18, read-only (sudo for reads only), PASS/WARN/FAIL, non-zero on any FAIL |
| `check-deploy.mjs` | The static checks for A14.6, A15.2 and the unit's hardening lines, with a selftest |

Also:
- `deny.toml`: the A14.8 policy.
- `.github/workflows/game-server.yml`: CI for the server, the A6 parity
  tests, and the deploy kit (podman build, image user, no shell, quadlet dry
  run, shellcheck, gitleaks, cargo-deny).

## Acceptance

| Test | Where it ran | Result |
|---|---|---|
| A14.1 `podman build` succeeds | CI workflow (Ubuntu runner) | **CI only**: no podman on this Mac |
| A14.2 the image runs as a non-root user | CI | **CI only** |
| A14.3 `--entrypoint sh` fails | CI | **CI only** |
| A14.4 `quadlet -dryrun -user` passes | CI, and runbook step 12 on Rocky | **CI only** |
| A14.5 / A15.1 shellcheck clean | CI | **CI only**: no shellcheck here |
| A14.6 the tunnel template has no admin port and has the 404 catch-all | `node server/deploy/check-deploy.mjs` | PASS (it caught "8081" in a comment, now reworded) |
| A14.7 no secrets | `check-deploy.mjs` pattern pass: PASS. Full gitleaks: **CI only** |
| A14.8 `cargo deny check` | CI | **CI only**: cargo-deny not installed here |
| A15.2 `verify-host.sh` contains no write | `check-deploy.mjs` (+ selftest proving each rule bites) | PASS |
| The unit is hardened and loopback-only, with no auto-update | `check-deploy.mjs` | PASS |
| `deploy.sh` refuses a non-loopback publish, auto-update, a placeholder TTS base, or a missing image | by reading the script | written; runs on Rocky |
| The server config: the container exception is off by default and explicit when on | `cargo test -p spell-server --lib config` | PASS |
| Every script parses (`bash -n`), pending shellcheck in CI | local | PASS |
| The whole server suite after the config change (81 unit, 16 socket, 3 process) and clippy | local | PASS |
| The app's 54 static checks | `scripts/hooks/pre-push` | PASS |

## Decisions for Eric

1. **The loopback rule (A4.2) and rootless Podman. Eric 2026-10-25: "Yes to SPELL_PUBLISHED_LOOPBACK_ONLY as built."** I flagged this in Phase
   A. A published port reaches the container on its own interface, so a
   server bound to `127.0.0.1` inside the container would likely be
   unreachable. I couldn't test Podman's forwarding here (no Linux, no
   podman), so I built the explicit version and need your yes:
   - The image listens on `0.0.0.0` inside its own network namespace.
   - The unit sets **`SPELL_PUBLISHED_LOOPBACK_ONLY=true`**. That is the only
     way the server accepts a non-loopback bind together with trusting
     `CF-Connecting-IP`, and it logs a warning at startup.
   - Without the setting, A4.2 refuses exactly as before (tested).
   - The guarantee moves to the host's publish line, and three things prove it
     there:
     - `check-deploy.mjs` rejects any `PublishPort` that isn't `127.0.0.1:`.
     - `deploy.sh` refuses such a unit.
     - `verify-host.sh` H5 fails if anything but sshd listens beyond
       loopback.

   **If you'd rather keep A4.2 literal,** the runbook's step 12 test shows
   whether Podman can reach a loopback-bound container on Rocky. If it can, I
   remove the setting.
2. **CI for the container half.** The new workflow is the only place A14.1–5,
   A14.7 and A14.8 run until the tools exist somewhere. Either let it run on
   push (it does once this branch is on main), or install podman, shellcheck,
   gitleaks and cargo-deny on Rocky, where the runbook already installs
   podman.

## Next

Phase E (load, soak and chaos through the real tunnel) needs the runbook done
on Rocky. When it is, `verify-host.sh` tells me which H-checks are still
missing.
