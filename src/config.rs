use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const DEFAULT_MIN_LETTER_HITS: usize = 3;
const DEFAULT_MAX_SELECTION_BYTES: usize = 1024 * 1024;
const DEFAULT_CLIPBOARD_DELAY_MS: u64 = 150;

fn default_min_letter_hits() -> usize {
    DEFAULT_MIN_LETTER_HITS
}
fn default_max_selection_bytes() -> usize {
    DEFAULT_MAX_SELECTION_BYTES
}
fn default_clipboard_delay_ms() -> u64 {
    DEFAULT_CLIPBOARD_DELAY_MS
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub primary: String,
    pub layouts: Vec<String>,
    #[serde(default)]
    pub tuning: Tuning,
}

#[derive(Deserialize, Clone)]
pub struct Tuning {
    #[serde(default = "default_min_letter_hits")]
    pub min_letter_hits: usize,
    #[serde(default = "default_max_selection_bytes")]
    pub max_selection_bytes: usize,
    #[serde(default = "default_clipboard_delay_ms")]
    pub clipboard_delay_ms: u64,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            min_letter_hits: DEFAULT_MIN_LETTER_HITS,
            max_selection_bytes: DEFAULT_MAX_SELECTION_BYTES,
            clipboard_delay_ms: DEFAULT_CLIPBOARD_DELAY_MS,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = get_config_path();
        let mut config = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                    eprintln!("invalid config format: {e}, falling back to defaults");
                    default_config()
                }),
                Err(e) => {
                    eprintln!("failed to read config: {e}, using defaults");
                    default_config()
                }
            }
        } else {
            // Write default config to disk so the user has a template to edit.
            let default_conf = default_config();
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let template = r#"# Primary layout used as the physical layout bridge
primary = "en"

# Active layouts. The order determines tie-breaker priority.
layouts = ["en", "ru"]

[tuning]
min_letter_hits = 3
max_selection_bytes = 1048576
clipboard_delay_ms = 150
"#;
            let _ = fs::write(&path, template);
            default_conf
        };

        // Support environment overrides for containerized or systemd setups.
        if let Ok(val) = std::env::var("LAYOUT_SWITCHER_MIN_LETTER_HITS") {
            if let Ok(parsed) = val.parse() {
                config.tuning.min_letter_hits = parsed;
            }
        }
        if let Ok(val) = std::env::var("LAYOUT_SWITCHER_MAX_SELECTION_BYTES") {
            if let Ok(parsed) = val.parse() {
                config.tuning.max_selection_bytes = parsed;
            }
        }
        if let Ok(val) = std::env::var("LAYOUT_SWITCHER_CLIPBOARD_DELAY_MS") {
            if let Ok(parsed) = val.parse() {
                config.tuning.clipboard_delay_ms = parsed;
            }
        }

        config
    }
}

fn get_config_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
            PathBuf::from(home).join(".config")
        })
        .join("layout-switcher")
        .join("config.toml")
}

fn default_config() -> Config {
    Config {
        primary: "en".to_string(),
        layouts: vec!["en".to_string(), "ru".to_string()],
        tuning: Tuning::default(),
    }
}
