use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone)]
pub struct LayoutData {
    pub name: String,
    pub keys: String,
}

pub struct Layout {
    pub name: String,
    pub keys: String,
    // Maps a character to its physical index in the 94-key sequence
    pub char_to_idx: HashMap<char, usize>,
}

pub struct LayoutRegistry {
    pub layouts: HashMap<String, Layout>,
    pub unique_chars: HashMap<String, HashSet<char>>,
}

impl LayoutRegistry {
    pub fn load(active_names: &[String]) -> Self {
        let mut registry = Self {
            layouts: HashMap::new(),
            unique_chars: HashMap::new(),
        };

        // Load built-in layouts from embedded files.
        let builtins = vec![
            include_str!("../layouts/en.toml"),
            include_str!("../layouts/ru.toml"),
            include_str!("../layouts/uk.toml"),
            include_str!("../layouts/de.toml"),
            include_str!("../layouts/by.toml"),
            include_str!("../layouts/kz.toml"),
            include_str!("../layouts/dvorak.toml"),
            include_str!("../layouts/colemak.toml"),
        ];

        for content in builtins {
            if let Ok(data) = toml::from_str::<LayoutData>(content) {
                registry.add_layout(data);
            }
        }

        // Load custom layouts from config directory.
        if let Some(custom_dir) = get_layouts_dir() {
            if custom_dir.exists() {
                if let Ok(entries) = fs::read_dir(custom_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                if let Ok(data) = toml::from_str::<LayoutData>(&content) {
                                    registry.add_layout(data);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate unique alphabetic characters for active layouts.
        let active_layouts: Vec<&Layout> = active_names
            .iter()
            .filter_map(|name| registry.layouts.get(name))
            .collect();

        for (i, current) in active_layouts.iter().enumerate() {
            let current_chars: HashSet<char> =
                current.keys.chars().filter(|c| c.is_alphabetic()).collect();

            let mut other_chars = HashSet::new();
            for (j, other) in active_layouts.iter().enumerate() {
                if i != j {
                    other_chars.extend(other.keys.chars().filter(|c| c.is_alphabetic()));
                }
            }

            let unique = current_chars
                .into_iter()
                .filter(|c| !other_chars.contains(c))
                .collect();

            registry.unique_chars.insert(current.name.clone(), unique);
        }

        registry
    }

    fn add_layout(&mut self, data: LayoutData) {
        let name = data.name.clone();
        let mut char_to_idx = HashMap::new();
        for (idx, c) in data.keys.chars().enumerate() {
            // Keep first key position for deduplication compatibility
            char_to_idx.entry(c).or_insert(idx);
        }

        self.layouts.insert(
            name.clone(),
            Layout {
                name,
                keys: data.keys,
                char_to_idx,
            },
        );
    }

    pub fn translate_between(&self, text: &str, from_name: &str, to_name: &str) -> Option<String> {
        let from = self.layouts.get(from_name)?;
        let to = self.layouts.get(to_name)?;

        let to_chars: Vec<char> = to.keys.chars().collect();
        let mut result = String::with_capacity(text.len());

        for c in text.chars() {
            if let Some(&idx) = from.char_to_idx.get(&c) {
                if let Some(&target_char) = to_chars.get(idx) {
                    result.push(target_char);
                    continue;
                }
            }
            result.push(c);
        }

        Some(result)
    }

    pub fn detect_source(
        &self,
        text: &str,
        active_names: &[String],
        min_hits: usize,
    ) -> Option<String> {
        let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();

        // 1. Look for unique character markers (unconditional match)
        for name in active_names {
            if let Some(uniques) = self.unique_chars.get(name) {
                if letters.iter().any(|c| uniques.contains(c)) {
                    return Some(name.clone());
                }
            }
        }

        // 2. Count match coverage
        let mut best_name = None;
        let mut max_hits = 0;

        for name in active_names {
            if let Some(layout) = self.layouts.get(name) {
                let hits = letters
                    .iter()
                    .filter(|c| layout.char_to_idx.contains_key(c))
                    .count();

                // Order in active_names config breaks ties (earlier layout wins)
                if hits > max_hits {
                    max_hits = hits;
                    best_name = Some(name.clone());
                }
            }
        }

        if max_hits < min_hits {
            return None;
        }

        best_name
    }
}

fn get_layouts_dir() -> Option<PathBuf> {
    let home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok()?;
    Some(home.join("layout-switcher").join("layouts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layouts_validity() {
        let active = vec![
            "en".to_string(),
            "ru".to_string(),
            "uk".to_string(),
            "de".to_string(),
            "by".to_string(),
            "kz".to_string(),
            "dvorak".to_string(),
            "colemak".to_string(),
        ];

        let registry = LayoutRegistry::load(&active);

        for name in &active {
            let layout = registry.layouts.get(name).expect("missing layout");
            assert_eq!(
                layout.keys.chars().count(),
                94,
                "layout {} must have exactly 94 chars",
                name
            );
        }
    }

    #[test]
    fn test_translate_between() {
        let active = vec!["en".to_string(), "ru".to_string(), "de".to_string()];
        let registry = LayoutRegistry::load(&active);

        assert_eq!(
            registry
                .translate_between("ghbdtn? vbh!", "en", "ru")
                .as_deref(),
            Some("привет, мир!")
        );

        assert_eq!(
            registry
                .translate_between("привет, мир!", "ru", "en")
                .as_deref(),
            Some("ghbdtn? vbh!")
        );

        assert_eq!(
            registry.translate_between("xyz", "en", "de").as_deref(),
            Some("xzy")
        );
    }

    #[test]
    fn test_detect_source() {
        let active = vec![
            "en".to_string(),
            "ru".to_string(),
            "uk".to_string(),
            "de".to_string(),
            "by".to_string(),
            "kz".to_string(),
        ];
        let registry = LayoutRegistry::load(&active);

        assert_eq!(
            registry.detect_source("привет", &active, 3),
            Some("ru".to_string())
        );
        assert_eq!(
            registry.detect_source("привіт", &active, 3),
            Some("uk".to_string())
        );
        assert_eq!(
            registry.detect_source("straße", &active, 3),
            Some("de".to_string())
        );
        assert_eq!(
            registry.detect_source("ў", &active, 3),
            Some("by".to_string())
        );
        assert_eq!(
            registry.detect_source("ә", &active, 3),
            Some("kz".to_string())
        );

        assert_eq!(
            registry.detect_source("hello world", &active, 3),
            Some("en".to_string())
        );
        // Under threshold should return None
        assert_eq!(registry.detect_source("ab", &active, 3), None);
    }
}
