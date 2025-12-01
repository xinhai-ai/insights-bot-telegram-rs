use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use serde_yaml::Value;
use tracing::info;

use crate::config::Locale;

// Embed locale files at compile time for self-contained binary
const EMBEDDED_EN: &str = include_str!("../locales/en.yml");
const EMBEDDED_ZH_HANS: &str = include_str!("../locales/zh-Hans.yml");
const EMBEDDED_ZH_HANT: &str = include_str!("../locales/zh-Hant.yml");

#[derive(Clone)]
pub struct I18n {
    bundles: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    /// Load locale bundles from embedded files (compile-time).
    /// Falls back to directory loading if specified.
    pub fn load_from_dir(dir: &str) -> Result<Self> {
        let embedded = [
            ("en", EMBEDDED_EN),
            ("zh-Hans", EMBEDDED_ZH_HANS),
            ("zh-Hant", EMBEDDED_ZH_HANT),
        ];

        // First try to load embedded locales (always available)
        let mut bundles = HashMap::new();

        for &(code, content) in &embedded {
            let value: Value = serde_yaml::from_str(content)?;
            let mut flat = HashMap::new();
            flatten_yaml(None, &value, &mut flat);
            bundles.insert(code.to_string(), flat);
        }

        info!("loaded {} embedded locale bundles", bundles.len());

        let dir_path = Path::new(dir);

        // Ensure locales directory exists and seed files if absent.
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
            info!("created locales directory at {}", dir_path.display());
        }
        for &(code, content) in &embedded {
            let path = dir_path.join(format!("{code}.yml"));
            if !path.exists() {
                fs::write(&path, content)?;
                info!("seeded default locale file {}", path.display());
            }
        }

        // Merge external files on top of embedded (external keys override embedded,
        // but new embedded keys not present in external files are preserved).
        for &(code, _) in &embedded {
            let path = dir_path.join(format!("{code}.yml"));
            if path.exists() {
                if let Ok(raw) = fs::read_to_string(&path) {
                    if let Ok(value) = serde_yaml::from_str::<Value>(&raw) {
                        let mut external_flat = HashMap::new();
                        flatten_yaml(None, &value, &mut external_flat);

                        // Merge: external keys override embedded keys
                        if let Some(base) = bundles.get_mut(code) {
                            let external_count = external_flat.len();
                            let base_count = base.len();
                            for (k, v) in external_flat {
                                base.insert(k, v);
                            }
                            let merged_count = base.len();
                            let new_keys = merged_count.saturating_sub(external_count);
                            if new_keys > 0 {
                                info!(
                                    "merged locale {code}: {} external keys + {} new embedded keys",
                                    external_count, new_keys
                                );
                            } else {
                                info!(
                                    "merged locale {code} from {} ({} keys)",
                                    path.display(),
                                    base_count
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(Self { bundles })
    }

    pub fn t(&self, locale: Locale, key: &str, args: &[(&str, &str)]) -> String {
        let code = locale.code();
        let template = self
            .bundles
            .get(code)
            .and_then(|m| m.get(key))
            .or_else(|| self.bundles.get("en").and_then(|m| m.get(key)))
            .unwrap_or(&key.to_string())
            .clone();

        apply_args(&template, args)
    }
}

fn flatten_yaml(prefix: Option<&str>, value: &Value, out: &mut HashMap<String, String>) {
    match value {
        Value::Mapping(map) => {
            for (k, v) in map {
                if let Some(k_str) = k.as_str() {
                    let next = prefix
                        .map(|p| format!("{p}.{k_str}"))
                        .unwrap_or_else(|| k_str.to_string());
                    flatten_yaml(Some(&next), v, out);
                }
            }
        }
        Value::Sequence(seq) => {
            for (idx, v) in seq.iter().enumerate() {
                let next = prefix
                    .map(|p| format!("{p}.{idx}"))
                    .unwrap_or_else(|| idx.to_string());
                flatten_yaml(Some(&next), v, out);
            }
        }
        Value::String(s) => {
            if let Some(k) = prefix {
                out.insert(k.to_string(), s.clone());
            }
        }
        Value::Null => {
            if let Some(k) = prefix {
                out.insert(k.to_string(), "null".to_string());
            }
        }
        Value::Bool(b) => {
            if let Some(k) = prefix {
                out.insert(k.to_string(), b.to_string());
            }
        }
        Value::Number(n) => {
            if let Some(k) = prefix {
                out.insert(k.to_string(), n.to_string());
            }
        }
        other => {
            if let Some(k) = prefix {
                out.insert(k.to_string(), format!("{other:?}"));
            }
        }
    }
}

fn apply_args(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}
