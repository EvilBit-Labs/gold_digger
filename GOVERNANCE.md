# Governance

Gold Digger is a **maintainer-driven** open source project under the [EvilBit-Labs](https://github.com/EvilBit-Labs) organisation.

## Roles

| Role                 | Holder                                         | Responsibilities                                                      |
| -------------------- | ---------------------------------------------- | --------------------------------------------------------------------- |
| **Maintainer**       | [@unclesp1d3r](https://github.com/unclesp1d3r) | Final authority on code, releases, roadmap, and community standards   |
| **Security Contact** | [@unclesp1d3r](https://github.com/unclesp1d3r) | Triage and remediation of vulnerabilities per `SECURITY.md`           |
| **Contributor**      | Anyone                                         | Submit issues, pull requests, documentation improvements, and reviews |

## Decision-Making Process

### Bug Fixes and Minor Changes

Bug fixes, documentation improvements, and minor refactors are reviewed and merged by the maintainer at their discretion.

### New Features

Feature proposals should start as a GitHub Issue for discussion. The maintainer evaluates scope, alignment with the [roadmap](ROADMAP.md), and maintenance burden before approving implementation.

### Breaking Changes

Breaking changes require:

1. A GitHub Issue describing the change and migration path
2. Explicit maintainer approval
3. A deprecation period when feasible (at least one minor release)
4. Updated documentation and changelog entries

### Releases

The maintainer controls the release cadence. See [RELEASING.md](RELEASING.md) for the release process.

## Becoming a Maintainer

This is currently a single-maintainer project. Consistent, high-quality contributions over time may lead to expanded roles as the project grows. If you are interested, open an issue to start a conversation.

## Continuity Plan

To ensure the project can survive maintainer unavailability:

- The repository is owned by the **EvilBit-Labs** organisation, not a personal account
- CI/CD pipelines are fully automated via GitHub Actions
- All release, security, and quality processes are documented in-repo
- Dependency updates are automated via Dependabot
- The `AGENTS.md` and `CLAUDE.md` files provide onboarding context for contributors and AI assistants
