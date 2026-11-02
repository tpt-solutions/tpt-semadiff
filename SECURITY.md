# Security Policy

## Supported versions

tpt-semadiff is pre-1.0. Only the latest release receives security fixes. Older releases
and unreleased `main` builds are not supported. Until 0.1.0 is published, there is no
release to support, so please report against the code on `master`.

## Reporting a vulnerability

Please report security problems privately, not in a public issue or pull request.

Use GitHub's private vulnerability reporting:

1. Go to https://github.com/tpt-solutions/tpt-semadiff/security
2. Choose **Report a vulnerability**.
3. Describe the problem, the version or commit you tested, and the steps to reproduce it.

Reports are visible only to the maintainers and to you. We will acknowledge a report as soon
as we can, and we will keep you informed while we work on a fix. Once a fix is released, we
will credit you in the release notes unless you ask us not to.

## Scope

In scope: the `semadiff` binary and the three library crates in this repository, including
how they read files, run git, and send data to the Anthropic API when `--summary llm` is
used. See [docs/privacy.md](docs/privacy.md) for what leaves the machine.

Out of scope: vulnerabilities in tree-sitter, git, or the Anthropic API themselves. Report
those to their maintainers.
