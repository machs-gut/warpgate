# machs-gut Warpgate downstream

This branch is a thin downstream of upstream Warpgate `v0.28.5` (`9711335bf80c3c48808d6e3aea0775e0a4720be8`).

## Scope

- Keep authentication, target selection, database schema, audit policy, and non-Web-SSH protocols aligned with upstream.
- Custom changes are limited to Web SSH terminal themes and an agentless Linux metrics side-channel carried over the existing authenticated SSH connection.
- The metrics collector must be fixed by the backend; the browser must never be able to submit arbitrary exec commands.
- Metrics channels must not be exposed as user shell tabs and must not create normal terminal recordings.
- No database schema migration is allowed for the first downstream release so rollback to upstream `0.28.5` stays possible.

## Versioning

Downstream release tags use `v0.28.5-linc.N`; container images are published as `ghcr.io/machs-gut/warpgate:0.28.5-linc.N`.

## Upstream sync

Rebase each downstream release onto an explicit upstream Warpgate tag. Keep downstream commits small and grouped by backend metrics, Web SSH UI/themes, tests, and release plumbing.

## Build policy

- Local Rust, frontend, and container image builds belong on `bom-wk01`; do not run compilation workloads on Kubernetes control-plane nodes such as `nue-cp02`.
- Release images are built on GitHub Hosted Runners, not on cluster nodes.
- `linux/amd64` uses the normal `release` profile with 8 Cargo jobs.
- `linux/arm64` uses the upstream `release-no-lto` profile with 2 Cargo jobs to stay within GitHub ARM runner memory limits; functionality is identical, only link-time optimization is disabled for that architecture.
- Control-plane nodes are limited to lightweight Git/deployment orchestration and verification commands.

## Dependency policy

- Follow dependency versions from explicit upstream Warpgate release tags; do not independently drift the downstream fork through automated version-update PRs.
- Dependabot version-update PRs are disabled for this downstream repository.
- Security findings are reviewed separately; urgent fixes can be backported deliberately when required.

## Web SSH UI

- The terminal theme defaults to Tokyo Night and can be switched between Tokyo Night, Catppuccin Mocha, Dracula, Nord, Warpgate Original, and GitHub Light. Light terminal themes also switch the Web SSH workspace chrome to a matching light appearance.
- Theme and font-size preferences are browser-local only (`warpgateWebSSHTheme`, `warpgateWebSSHFontSize`) and do not change the Warpgate database.
- Web SSH starts the fixed backend metrics collector automatically and keeps only the latest 60 one-second samples in browser memory for sparklines.
- The active session metrics render in a vertical rail; CPU, memory, normalized load, and root disk usage gain warning/critical emphasis, and samples older than 3.5 seconds are marked `STALE`.
- Top-level tabs represent server sessions; additional shell channels on one server use a secondary shell strip and still prefer remote OSC terminal titles when available.
- The metrics rail shows CPU, memory, network RX/TX, load, root filesystem usage, uptime, CPU count, and the selected network interface.
- Web SSH can host multiple independent target sessions in one browser workspace; every target gets its own server-created Web SSH session and authenticated WebSocket rather than sharing a shell or jump-host process.
- The workspace layout uses a collapsible SSH target tree on the left, server session tabs across the top, the active terminal in the center, and a collapsible vertical metrics rail on the right.
- Target checkboxes are only a browser-side batch selector; connecting still calls the normal `create_web_ssh_session` API for every target so existing target permissions, authentication, audit, and recording boundaries remain intact.
- Metrics stay scoped to the active target session. Switching server tabs switches the metrics rail with it; background sessions retain only their own 60-sample browser history.
- Multiple shell channels on one server remain supported and appear as a small secondary shell strip only when more than one channel exists.
- Sidebar, metrics-rail, terminal-theme, and font-size preferences are browser-local; workspace target sessions are intentionally not persisted across page reloads.

- Web SSH buffers up to 1 MiB of early terminal output per shell channel until the xterm instance is mounted, then flushes and refits it; this prevents fast remote login banners/prompts from being lost during workspace session creation.
