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

- Run Rust, frontend, and container image builds on `bom-wk01`; do not run compilation workloads on Kubernetes control-plane nodes such as `nue-cp02`.
- Control-plane nodes are limited to lightweight Git/deployment orchestration and verification commands.
