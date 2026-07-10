# powerline-claude

A powerline-style status line for [Claude Code](https://code.claude.com), as a
single Rust binary. Reads the statusline JSON Claude Code writes to stdin,
prints an ANSI bar.

![the status line, default catppuccin-mocha theme](docs/statusline.png)

Inspired by [starship-claude](https://github.com/martinemde/starship-claude),
using a [powerline-go](https://github.com/justjanne/powerline-go)-style API.

## Quick start

These are Claude Code slash commands, not shell commands: start a `claude`
session and enter each one _at the Claude prompt_, one at a time.

`/plugin marketplace add bcmyguest/powerline-claude`

`/plugin install powerline-claude@powerline-claude`

`/reload-plugins`

Then run the setup command (also inside Claude):

`/powerline-claude:configure`

It downloads the binary if it's missing, points `statusLine` in
`~/.claude/settings.json` at it, and walks you through theme, segments, and
separator mode.

## Manual install

Grab the static binary from the latest release:

```bash
curl -fsSL -o ~/.local/bin/powerline-claude \
  https://github.com/bcmyguest/powerline-claude/releases/latest/download/powerline-claude-x86_64-unknown-linux-musl
chmod +x ~/.local/bin/powerline-claude
```

or build from source:

```bash
cargo install --git https://github.com/bcmyguest/powerline-claude --locked
```

## Usage

`~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "padding": 0,
    "command": "~/.local/bin/powerline-claude"
  }
}
```

Flags go on that command string:

| Flag | Default | Meaning |
|------|---------|---------|
| `--modules` | `logo,dir,git,model,context,cost,stats,effort` | Segments to render, in order |
| `--theme` | `catppuccin-mocha` | Also: `catppuccin-frappe`, `dracula`, `gruvbox-dark`, `nord`, `tokyonight` |
| `--mode` | `patched` | `patched` (nerd-font separators), `compatible` (plain Unicode), `flat` (none) |
| `--no-progress` | off | Suppress the OSC 9;4 terminal progress bar |
| `--width` | `$COLUMNS`, then parent TTY, then 200 | Terminal width (drives dir truncation) |

## Segments

- `logo` — Claude glyph
- `dir` — workspace dir, last two path components (one below 80 columns)
- `git` — current branch (read from `.git/HEAD`, worktree-aware) plus the
  session's `+added -removed` line counts from the payload
- `model` — nerd icon + lowercased model name
- `context` — exact tokens in the context window (`150,697 tok`), `~~ tok`
  before the first API call
- `cost` — session cost, `$X.XX`
- `stats` — session duration (`1h 12m`)
- `effort` — reasoning effort level; hidden when the model doesn't support it

Segments whose data is absent from the payload disappear rather than render
placeholders (except `context`, which shows `~~` like the old bar did).

The OSC 9;4 progress bar mirrors context usage: green below 40%, yellow to
60%, red above, full at the 80% compact threshold.

## Development

```bash
cargo test          # the suite: unit + fixture-driven integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo build --release
```

Rendering is pure (`powerline_claude::run`: JSON in, ANSI out), so everything
is testable without a terminal; fixtures live in `tests/fixtures/`.

## Releasing

Releases are automatic. Every merge to `main` runs
`.github/workflows/release.yml`, which asks [git-cliff](https://git-cliff.org)
for the next semver based on the conventional commits since the last tag,
pushes that `vX.Y.Z` tag, builds the static `x86_64-unknown-linux-musl`
binary, and publishes a GitHub release with a git-cliff changelog
(`feat` → minor, breaking → major, anything else → patch). The committed
`Cargo.toml` version is not bumped; the binary is stamped with the tag
version at build time.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org),
enforced by a [pre-commit](https://pre-commit.com) `commit-msg` hook:

```bash
pre-commit install --install-hooks --hook-type pre-commit --hook-type commit-msg
```

## Plugin

`plugin/` is a small Claude Code plugin (registered via the repo-root
`.claude-plugin/marketplace.json`) providing `/powerline-claude:configure`:
an interactive way to pick a theme, choose and order segments, or change the
separator mode — it previews candidates by piping a sample payload through
the binary, then rewrites the `statusLine.command` flags.

## License

AGPL-3.0-only.
