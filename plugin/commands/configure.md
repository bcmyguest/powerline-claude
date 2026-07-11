---
description: Configure the powerline-claude status line (theme, modules, separator mode)
---

Reconfigure the powerline-claude Claude Code status line. The binary lives at
`~/.local/bin/powerline-claude`; its flags are the whole configuration surface
and they live on the `statusLine.command` string in `~/.claude/settings.json`.

User request (may be empty): $ARGUMENTS

Steps:

1. Verify `~/.local/bin/powerline-claude --help` runs. If the binary is
   missing, install the release asset for this platform:

   ```bash
   case "$(uname -sm)" in
     "Linux x86_64")   target=x86_64-unknown-linux-musl ;;
     "Linux aarch64")  target=aarch64-unknown-linux-musl ;;
     "Darwin arm64")   target=aarch64-apple-darwin ;;
     "Darwin x86_64")  target=x86_64-apple-darwin ;;
     *)                target= ;;
   esac
   mkdir -p ~/.local/bin
   curl -fsSL -o ~/.local/bin/powerline-claude \
     "https://github.com/bcmyguest/powerline-claude/releases/latest/download/powerline-claude-$target"
   chmod +x ~/.local/bin/powerline-claude
   ```

   On an unmatched platform (or if the download fails) fall back to building
   from source when cargo is available:
   `cargo install --git https://github.com/bcmyguest/powerline-claude --locked
   --root ~/.local`. If neither works, tell the user and stop.
2. Read the current `statusLine` entry from `~/.claude/settings.json` and show
   the user their current flags. If there is no `statusLine` (fresh install),
   set it — merge into the existing JSON, preserving every other key:

   ```json
   {
     "statusLine": {
       "type": "command",
       "padding": 0,
       "command": "~/.local/bin/powerline-claude"
     }
   }
   ```

   On a fresh install ask whether the user's terminal has a Nerd Font
   (default `--mode patched` needs one; otherwise add `--mode compatible`).
3. Ask what they want to change (unless $ARGUMENTS already says), offering:
   - `--theme`: catppuccin-mocha (default), catppuccin-frappe, dracula,
     gruvbox-dark, nord, tokyonight; also a path to a custom theme
     directory, or the bare name of one under
     `~/.config/powerline-claude/themes/`
   - `--modules`: any order/subset of
     logo,dir,git,model,context,cost,usage,stats,effort
   - `--modules-right`: same values, pinned to the right edge of the terminal
     (default: none — everything renders on the left)
   - `--mode`: patched (nerd font), compatible (plain-Unicode separators
     and segment icons, for terminals without a patched font), flat (no
     separators)
   - `--no-progress`: disable the terminal progress bar
4. To preview a candidate configuration, pipe a sample payload through the
   binary and show the raw output (the user's terminal renders the ANSI):

   ```bash
   echo '{"workspace":{"current_dir":"'"$PWD"'"},"model":{"display_name":"Opus 4.8"},"cost":{"total_cost_usd":0.71,"total_duration_ms":4335000,"total_lines_added":156,"total_lines_removed":23},"context_window":{"context_window_size":200000,"current_usage":{"input_tokens":8500,"output_tokens":1200,"cache_creation_input_tokens":5000,"cache_read_input_tokens":2000}},"effort":{"level":"high"}}' \
     | ~/.local/bin/powerline-claude --no-progress --theme <candidate> --modules <candidate>
   ```

5. Apply by updating only `statusLine.command` in `~/.claude/settings.json`
   (preserve every other key — read, merge, write back). Omit flags that match
   the defaults to keep the command string short.
6. Remind the user the change shows up on the next status line refresh.
