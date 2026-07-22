mod config;
mod layout;

use std::borrow::Cow;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use signal_hook::consts::{SIGUSR1, SIGUSR2};
use signal_hook::iterator::Signals;

use config::Config;
use layout::LayoutRegistry;

struct State {
    last_text: String,
    last_translation: String,
    last_time: Option<Instant>,
    detected_source: Option<String>,
    current_target_index: usize,
}

impl State {
    fn new() -> Self {
        Self {
            last_text: String::new(),
            last_translation: String::new(),
            last_time: None,
            detected_source: None,
            current_target_index: 0,
        }
    }

    fn is_cycle(&self, raw: &str, now: Instant) -> bool {
        self.last_time.is_some_and(|t| {
            now.duration_since(t) < Duration::from_secs(2)
                && (raw == self.last_translation.as_str() || raw == self.last_text.as_str())
        })
    }
}

fn collapse_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_newline = false;

    for c in text.chars() {
        if c == '\n' || c == '\r' {
            if !at_newline {
                out.push('\n');
                at_newline = true;
            }
        } else {
            out.push(c);
            at_newline = false;
        }
    }

    out
}

fn copy_to_clipboard(text: &str) -> bool {
    let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() else {
        return false;
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };

    let written = stdin.write_all(text.as_bytes()).is_ok();
    drop(stdin);

    if !written {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }

    matches!(child.wait(), Ok(status) if status.success())
}

fn read_clipboard() -> Option<String> {
    let out = Command::new("wl-paste").arg("--no-newline").output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

fn type_direct(text: &str) -> bool {
    let Ok(mut child) = Command::new("wtype").arg("-").stdin(Stdio::piped()).spawn() else {
        return false;
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };

    let written = stdin.write_all(text.as_bytes()).is_ok();
    drop(stdin);

    if !written {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }

    matches!(child.wait(), Ok(status) if status.success())
}

fn paste_via_clipboard(text: &str, delay_ms: u64) {
    let saved = read_clipboard();

    if !copy_to_clipboard(text) {
        eprintln!("wl-copy failed, nothing pasted");
        return;
    }

    std::thread::sleep(Duration::from_millis(delay_ms));

    let pasted = matches!(
        Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .status(),
        Ok(status) if status.success()
    );

    if !pasted {
        eprintln!("wtype ctrl+v failed");
    }

    if let Some(saved) = saved {
        std::thread::sleep(Duration::from_millis(delay_ms));
        if !copy_to_clipboard(&saved) {
            eprintln!("failed to restore clipboard");
        }
    }
}

fn process(collapse: bool, config: &Config, registry: &LayoutRegistry, state: &mut State) {
    let out = match Command::new("wl-paste")
        .args(["-p", "--no-newline"])
        .output()
    {
        Ok(out) => out,
        Err(e) => {
            eprintln!("failed to run wl-paste: {e}");
            return;
        }
    };

    if !out.status.success() {
        eprintln!("wl-paste exited with {}", out.status);
        return;
    }

    if out.stdout.len() > config.tuning.max_selection_bytes {
        eprintln!(
            "selection is {} bytes, over the {} byte limit, skipping",
            out.stdout.len(),
            config.tuning.max_selection_bytes
        );
        return;
    }

    let raw = match String::from_utf8(out.stdout) {
        Ok(raw) => raw,
        Err(_) => {
            eprintln!("selection is not valid UTF-8, skipping");
            return;
        }
    };

    let raw = if collapse {
        Cow::Owned(collapse_newlines(&raw))
    } else {
        Cow::Borrowed(raw.as_str())
    };

    if raw.is_empty() {
        eprintln!("selection is empty, nothing to do");
        return;
    }

    let now = Instant::now();
    let is_cycle = state.is_cycle(&raw, now);

    let (source_name, target_name) = if is_cycle {
        // Cycle mode: increment target layout index across all configured layouts
        let source_name = match state.detected_source.as_ref() {
            Some(name) => name,
            None => return,
        };

        let targets = &config.layouts;
        if targets.is_empty() {
            return;
        }

        state.current_target_index = (state.current_target_index + 1) % targets.len();
        let target_name = &targets[state.current_target_index];

        (source_name.clone(), target_name.clone())
    } else {
        // First press mode: detect source layout using min_letter_hits
        let source_name =
            match registry.detect_source(&raw, &config.layouts, config.tuning.min_letter_hits) {
                Some(name) => name,
                None => {
                    eprintln!("could not detect source layout, skipping");
                    return;
                }
            };

        let targets = &config.layouts;
        if targets.is_empty() {
            return;
        }

        // Default target: primary layout, or the first active secondary
        let target_name = if source_name != config.primary {
            &config.primary
        } else {
            targets
                .iter()
                .find(|&name| name != &source_name)
                .unwrap_or(&config.primary)
        };

        let target_idx = targets
            .iter()
            .position(|name| name == target_name)
            .unwrap_or(0);

        state.last_text = raw.to_string();
        state.detected_source = Some(source_name.clone());
        state.current_target_index = target_idx;

        (source_name, target_name.clone())
    };

    let text_to_translate = if is_cycle {
        &state.last_text
    } else {
        raw.as_ref()
    };

    let translated = match registry.translate_between(text_to_translate, &source_name, &target_name)
    {
        Some(t) => t,
        None => {
            eprintln!("translation from {source_name} to {target_name} failed");
            return;
        }
    };

    state.last_translation = translated.clone();
    state.last_time = Some(now);

    if !type_direct(&translated) {
        eprintln!("direct wtype failed, falling back to clipboard paste");
        paste_via_clipboard(&translated, config.tuning.clipboard_delay_ms);
    }
}

fn main() {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        eprintln!("WAYLAND_DISPLAY is not set, wl-copy/wtype won't work");
        std::process::exit(1);
    }

    let config = Config::load();
    let registry = LayoutRegistry::load(&config.layouts);
    let mut state = State::new();

    let mut signals = Signals::new([SIGUSR1, SIGUSR2]).expect("can't install signal handlers");

    for sig in signals.forever() {
        process(sig == SIGUSR2, &config, &registry, &mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collapse_newlines() {
        let text = "line1\n\n\nline2\r\n\r\nline3";
        assert_eq!(collapse_newlines(text), "line1\nline2\nline3");
    }

    #[test]
    fn collapse_newlines_handles_standalone_cr() {
        assert_eq!(
            collapse_newlines("line1\rline2\r\rline3"),
            "line1\nline2\nline3"
        );
    }

    #[test]
    fn test_state_cycle_logic() {
        let mut state = State::new();
        let now = Instant::now();

        assert!(!state.is_cycle("привет", now));

        state.last_text = "ghbdtn".to_string();
        state.last_translation = "привет".to_string();
        state.last_time = Some(now);

        let now2 = Instant::now();
        // Cycles when raw equals last translation
        assert!(state.is_cycle("привет", now2));
        // Cycles when raw equals last text (robustness fix)
        assert!(state.is_cycle("ghbdtn", now2));

        let now3 = now + Duration::from_secs(3);
        assert!(!state.is_cycle("привет", now3));
    }
}
