use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use serde_yaml::Value;

use crate::config::Locale;

#[derive(Clone)]
pub struct I18n {
    bundles: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    pub fn load_from_dir(dir: &str) -> Result<Self> {
        let mut bundles = HashMap::new();
        for code in ["en", "zh-Hans", "zh-Hant"] {
            let path = Path::new(dir).join(format!("{code}.yml"));
            if !path.exists() {
                continue;
            }
            let raw = fs::read_to_string(&path)?;
            let value: Value = serde_yaml::from_str(&raw)?;
            let mut flat = HashMap::new();
            flatten_yaml(None, &value, &mut flat);
            bundles.insert(code.to_string(), flat);
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
