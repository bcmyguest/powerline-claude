# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

powerline-claude is a single Rust binary that renders a powerline-style status line for Claude Code: it reads the statusline JSON Claude Code writes to stdin and prints an ANSI bar to stdout.

## Commands

```bash
cargo test                                  # full suite: unit + fixture-driven integration tests
cargo test --test render                    # one integration suite (tests/render.rs)
cargo test theme::                          # unit tests by module path
cargo clippy --all-targets -- -D warnings   # CI fails on any warning
cargo fmt --check
cargo build --release
```

Run the binary by hand against a fixture:

```bash
./target/release/powerline-claude --width 200 < tests/fixtures/full.json
```

Pre-commit enforces Conventional Commits (`feat:`, `fix:`, `docs:`, …) via a commit-msg hook; install with `pre-commit install --install-hooks --hook-type pre-commit --hook-type commit-msg`. Commit types drive release versioning (see below), so pick them deliberately.

## Architecture

The whole program is `powerline_claude::run` in `src/lib.rs`: JSON in, ANSI string out, no I/O besides theme-file reads and `.git/HEAD`. This purity is the core design constraint — everything is testable without a terminal, and the integration tests in `tests/` drive `run` with the JSON fixtures in `tests/fixtures/`.

Data flow: `payload.rs` (serde deserialization; **every field is `Option`** because the payload varies across Claude Code versions) → `segments.rs` (module list → `(SegmentKind, text)` pairs; segments with absent payload data are dropped rather than rendered as placeholders, except `context` which shows `~~ tok`) → `theme.rs` (maps each `SegmentKind` to fg/bg `Rgb`) → `render.rs` (joins segments with powerline separators; hard separator between different backgrounds, thin within the same background).

`main.rs` is only the shim around `run`: stdin/stdout wiring, the parent-TTY width fallback (via `ps`/`stty`) for Claude Code < 2.1.153 where `$COLUMNS` is unset, and the OSC 9;4 progress bar (`progress.rs`), which is written straight to `/dev/tty` because Claude Code captures stdout.

Constraints to preserve:

- **The statusline runs on every render, so the library never spawns subprocesses.** The git branch is read from `.git/HEAD` directly (worktree-aware, walking up from the payload dir). Subprocess use is allowed only in `main.rs`'s one-time width fallback.
- **Themes:** built-in palettes live in `theme.rs::PALETTES`; `catppuccin-mocha` (`PALETTES[0]`) is the fallback for every unspecified custom-theme value. A custom theme is a directory containing `theme.yaml` (any subset of the six families), selected by passing a path to `--theme`. `docs/themes/synthwave` is the documented example. `stats` and `effort` have no palette entries of their own — they derive from `cost`/`context` and `model` in `Theme::colors`.
- **Flag surface is duplicated in three places.** When adding or changing a CLI flag, module, or theme, update all of: `src/cli.rs`, the README flags table / theme list, and the plugin's `plugin/commands/configure.md` (the `/powerline-claude:configure` command re-writes `statusLine.command` flags and lists valid values inline).

## Releases

Every merge to `main` runs `.github/workflows/release.yml`: git-cliff computes the next semver from conventional commits (`feat` → minor, breaking → major, else patch), pushes the `vX.Y.Z` tag, builds the static musl binary, publishes a GitHub release, and publishes to crates.io via Trusted Publishing (OIDC; the crate's crates.io settings must list this repo + `release.yml` as a trusted publisher or the publish step fails the run — the tag and GitHub release land first regardless). Running the workflow manually (`workflow_dispatch`) skips the release half and (re)publishes the latest existing tag to crates.io, e.g. to backfill after a failed publish; it exits early if that version is already published. The committed `Cargo.toml` version is never bumped — the workflow seds the tag version in at build time. Don't bump the version in PRs.

## Plugin

`plugin/` is a Claude Code plugin (registered by the repo-root `.claude-plugin/marketplace.json`) whose single command `/powerline-claude:configure` installs the binary if missing and interactively rewrites the `statusLine.command` flags in `~/.claude/settings.json`. Plugin versions in `plugin/.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` are managed by hand.
