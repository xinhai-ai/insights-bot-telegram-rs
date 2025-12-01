use std::fs;

use insights_bot_telegram_rs::{config::Locale, i18n::I18n};

#[test]
fn i18n_loads_and_translates() {
    // prepare temp dir with minimal locale files
    let dir = tempfile::tempdir().unwrap();
    let en_path = dir.path().join("en.yml");
    fs::write(&en_path, "greet: \"Hello {name}\"").unwrap();

    let i18n = I18n::load_from_dir(dir.path().to_str().unwrap()).unwrap();
    let out = i18n.t(Locale::En, "greet", &[("name", "World")]);
    assert_eq!(out, "Hello World");

    // missing key should return key
    let fallback = i18n.t(Locale::En, "missing", &[]);
    assert_eq!(fallback, "missing");
}
