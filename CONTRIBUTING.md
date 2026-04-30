# Contributing to SClaw
## Development workflow
1. Fork and create a feature branch.
2. Make focused changes (one concern per PR).
3. Run formatting, checks, and tests locally.
4. Open a PR with:
   - problem statement
   - design summary
   - safety impact (policy/remediation changes)
   - test evidence

## Recommended local commands
```powershell path=null start=null
cargo fmt
cargo check
cargo test
```

## Coding standards
- Prefer explicit enums/structs over loosely typed maps for core workflow logic.
- New remediation paths must include rollback behavior.
- Policy checks should default to safe behavior (block or require approval).
- Avoid hidden side effects in tool adapters; capture behavior in evidence/audit trails.

## Issue labels
- `good-first-issue`: scoped, low-risk onboarding tasks
- `adapter`: observability/provider integrations
- `policy`: guardrail and safety logic
- `orchestration`: state machine and planning behavior
- `documentation`: README, examples, guides

## Security and safety
- Do not hardcode credentials.
- Use environment variables for tokens and endpoints.
- Treat production write actions as approval-gated by default.

## Release expectations
- Semver for releases.
- Changelog entries for behavior changes in policy, orchestration, and adapters.
