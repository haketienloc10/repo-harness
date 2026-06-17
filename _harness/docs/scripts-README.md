# Scripts

This directory contains harness automation tools.

## Harness CLI

The Rust Harness CLI is the primary interface for the durable layer. Installed
projects use the prebuilt binary at `_harness/bin/harness-cli` on macOS/Linux or
`_harness/bin/harness-cli.exe` on Windows for normal Harness work.

```bash
_harness/bin/harness-cli init          # Create the database
_harness/bin/harness-cli intake ...    # Record a feature intake classification
_harness/bin/harness-cli story ...     # Add or update a story (test matrix row)
_harness/bin/harness-cli story update --id US-001 --unit 1 --integration 1 --e2e 0 --platform 0
_harness/bin/harness-cli story verify US-001  # Run the story's verify_command
_harness/bin/harness-cli decision ...  # Add a decision or run its verification
_harness/bin/harness-cli backlog ...   # Add or close a backlog item
_harness/bin/harness-cli trace ...     # Record and auto-score an agent execution trace
_harness/bin/harness-cli score-trace   # Score a trace against TRACE_SPEC.md tiers
_harness/bin/harness-cli query ...     # Query harness data, including backlog --open/--closed
_harness/bin/harness-cli query matrix --numeric  # Show proof flags as 1/0
_harness/bin/harness-cli migrate       # Apply pending schema migrations
_harness/bin/harness-cli --version     # Print the installed CLI version
```

Run `_harness/bin/harness-cli help` or `_harness/bin/harness-cli query help` for
full usage. On Windows, use the same commands through
`.\scripts\bin\harness-cli.exe`.

Proof flags on `story update` are numeric booleans: use `1` for yes and `0` for
no. `story verify <id>` runs the configured `verify_command`; it does not accept
proof flags. Configure the command with `story add/update --verify`, run
`story verify <id>`, then update proof flags with `story update`.

Backlog `--risk` uses Harness lanes, not severity words: use `tiny`, `normal`,
or `high-risk`. Use `tiny` instead of `low`. `query matrix` defaults to
human-readable `yes`/`no`; use `query matrix --numeric` when copying values into
`story update`.

The schema lives in `_harness/schema/` and is version-controlled. The database
file (`harness.db`) is `.gitignore`d.

Requires: the prebuilt Rust CLI at `_harness/bin/harness-cli` on macOS/Linux or
`_harness/bin/harness-cli.exe` on Windows.

Direct database inspection may still use SQLite tools, but normal Harness use
should go through the Rust CLI.

### Rust CLI Commands

Current migrated commands:

```bash
_harness/bin/harness-cli init
_harness/bin/harness-cli migrate
_harness/bin/harness-cli import brownfield
_harness/bin/harness-cli intake ...
_harness/bin/harness-cli story add ...
_harness/bin/harness-cli story update ...
_harness/bin/harness-cli story verify ...
_harness/bin/harness-cli decision add ...
_harness/bin/harness-cli decision verify ...
_harness/bin/harness-cli backlog add ...
_harness/bin/harness-cli backlog close ...
_harness/bin/harness-cli trace ...
_harness/bin/harness-cli score-trace
_harness/bin/harness-cli query matrix
_harness/bin/harness-cli query backlog
_harness/bin/harness-cli query decisions
_harness/bin/harness-cli query intakes
_harness/bin/harness-cli query traces
_harness/bin/harness-cli query friction
_harness/bin/harness-cli query stats
_harness/bin/harness-cli query sql ...
```

`_harness/bin/harness-cli import brownfield` seeds or refreshes the durable
database from existing Harness v0 markdown in `_harness/docs/TEST_MATRIX.md`,
`docs/decisions/`, and `_harness/docs/HARNESS_BACKLOG.md`. This keeps already-installed
Harness repos on the Rust CLI path without losing their populated operating
docs.

## Installer

The upstream installer applies the Harness v0 operating files and folder
structure to a target project directory. It defaults to the current directory,
accepts a target path, and asks interactive users whether to `1. Merge`,
`2. Override`, or `3. Stop` when the target already contains `AGENTS.md`,
`docs/`, or `scripts/`. Non-interactive installs stop on those protected paths
unless `--merge` or `--override` is provided. Use `--merge` as the safe update
path for repositories that already have Harness: it keeps existing files in
place and creates only missing Harness files. Add `--refresh-agent-shim` when an
older install has the full generated Harness guide in `AGENTS.md` and should
move to the small stable shim. Use `--override` only when replacing the
protected Harness surface is intentional.

```bash
curl -fsSL "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.sh?$(date +%s)" | bash -s -- --yes
```

```powershell
& ([scriptblock]::Create((irm "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.ps1"))) -Yes
```

```bash
curl -fsSL "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.sh?$(date +%s)" | bash -s -- --merge --yes
```

```powershell
& ([scriptblock]::Create((irm "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.ps1"))) -Merge -Yes
```

```bash
curl -fsSL "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.sh?$(date +%s)" | bash -s -- --merge --refresh-agent-shim --yes
```

```powershell
& ([scriptblock]::Create((irm "https://raw.githubusercontent.com/haketienloc10/repo-harness/main/scripts/install-harness.ps1"))) -Merge -RefreshAgentShim -Yes
```

`--refresh-agent-shim` backs up `AGENTS.md` before changing it. If the existing
file is recognized as the old Harness-generated operating guide, the installer
replaces it with the current shim. Otherwise it appends or replaces only the
marked `<!-- HARNESS:BEGIN -->` block so project-specific instructions remain in
place.

The installer must stay limited to harness files. Do not use it to scaffold
application source folders, package scripts, CI, tests, platform shells, or fake
validation commands. The installer script is not part of the installed project
payload.

By default the installer also downloads the prebuilt Rust Harness CLI for the
current platform into `_harness/bin/harness-cli` on macOS/Linux or
`_harness/bin/harness-cli.exe` on Windows, then verifies its `.sha256` checksum.
A source branch can pin the release used by the installer through
`scripts/harness-cli-release-tag`; Phase 3 pins `harness-cli-v0.1.4` so branch
installs receive a Phase 3-built CLI. Set `HARNESS_CLI_RELEASE_TAG` to override
that tag, or set `HARNESS_CLI_BASE_URL` to point at an alternate artifact
directory, such as a local `file:///.../dist` directory created by
`scripts/build-harness-cli-release.sh`.

## Schema Migrations

Migration files live under `_harness/schema/` and are named `NNN-description.sql`
where `NNN` is a zero-padded version number. Run
`_harness/bin/harness-cli migrate` to apply pending migrations.

## Future Command Contract

Expected future checks:

```text
validate:quick
  format, lint, typecheck, unit tests, architecture check

test:integration
  backend contract and integration checks

test:e2e
  user-visible end-to-end flows

test:platform
  platform shell smoke checks, if the project has a native shell

test:release
  full suite, log checks, and performance smoke
```

## Release Packaging

Build the current-platform Rust CLI release artifact from the source repo:

```bash
scripts/build-harness-cli-release.sh
```

The script writes `dist/harness-cli-<platform>` plus `.sha256` checksums. The
Windows artifact includes the `.exe` suffix. Supported labels are:

- `macos-arm64`
- `macos-x64`
- `linux-x64`
- `linux-arm64`
- `windows-x64`

For cross-compilation, pass a Cargo target triple:

```bash
scripts/build-harness-cli-release.sh --target x86_64-unknown-linux-gnu
```

GitHub releases are produced by `.github/workflows/harness-cli-release.yml`.
Push a tag matching `v*` or `harness-cli-v*` to run the verification job, build
all supported targets on native hosted runners, and upload these release assets:

- `harness-cli-macos-arm64`
- `harness-cli-macos-arm64.sha256`
- `harness-cli-macos-x64`
- `harness-cli-macos-x64.sha256`
- `harness-cli-linux-x64`
- `harness-cli-linux-x64.sha256`
- `harness-cli-linux-arm64`
- `harness-cli-linux-arm64.sha256`
- `harness-cli-windows-x64.exe`
- `harness-cli-windows-x64.exe.sha256`
