//! Binary shim: stdin, stdout, and `/dev/tty` wiring around
//! `powerline_claude::run` — no decisions of its own.

use std::io::{Read, Write};
use std::process::Command;

use clap::Parser;

use powerline_claude::{Env, run};

fn main() {
    let cli = powerline_claude::cli::Cli::parse();

    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    match run(&input, &cli, &Env::from_process(), parent_tty_width) {
        Ok(output) => {
            print!("{}", output.bar);
            let _ = std::io::stdout().flush();
            // Stdout is captured by Claude Code; the progress escape must go
            // straight to the terminal.
            if let Some(progress) = output.progress
                && let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty")
            {
                let _ = tty.write_all(progress.as_bytes());
            }
        }
        Err(err) => print!("powerline-claude: {err}"),
    }
}

/// Width of the terminal the parent process is attached to. The statusline
/// subprocess has no TTY of its own, so ask `ps` for the parent's. Only
/// called by `resolve_width` when `--width` and `$COLUMNS` are both absent
/// (Claude Code < 2.1.153).
fn parent_tty_width() -> Option<usize> {
    let ppid = command_line(&["-o", "ppid=", "-p", &std::process::id().to_string()])?;
    let tty = command_line(&["-o", "tty=", "-p", &ppid])?;
    if tty.is_empty() || tty.starts_with('?') {
        return None;
    }
    let size = Command::new("stty")
        .args(["size", "-F", &format!("/dev/{tty}")])
        .output()
        .ok()?;
    let size = String::from_utf8(size.stdout).ok()?;
    size.split_whitespace().nth(1)?.parse().ok()
}

fn command_line(ps_args: &[&str]) -> Option<String> {
    let output = Command::new("ps").args(ps_args).output().ok()?;
    let line = String::from_utf8(output.stdout).ok()?;
    let line = line.trim();
    (!line.is_empty()).then(|| line.to_string())
}
