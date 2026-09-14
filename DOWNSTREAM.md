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

- The terminal theme defaults to Tokyo Night and can be switched between Tokyo Night, Catppuccin Mocha, Dracula, Nord, and Warpgate Original.
- Theme and font-size preferences are browser-local only (`warpgateWebSSHTheme`, `warpgateWebSSHFontSize`) and do not change the Warpgate database.
- Web SSH starts the fixed backend metrics collector automatically and keeps only the latest 60 one-second samples in browser memory for sparklines.
- The metrics bar shows CPU, memory, network RX/TX, load, root filesystem usage, uptime, CPU count, and the selected network interface.
