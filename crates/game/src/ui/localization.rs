use std::{borrow::Cow, collections::BTreeMap, fs, path::Path, sync::OnceLock};

use serde::Deserialize;

use crate::scenes::Language;

pub const DEFAULT_LOCALIZATION_PATH: &str = "assets/data/ui/localization.ron";

static LOCALIZATION: OnceLock<LocalizationDocument> = OnceLock::new();

#[derive(Default, Deserialize)]
#[serde(default)]
struct LocalizationDocument {
    strings: BTreeMap<String, LocalizedString>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct LocalizedString {
    english: String,
    chinese: String,
}

pub fn text(
    language: Language,
    key: &str,
    english: &'static str,
    chinese: &'static str,
) -> Cow<'static, str> {
    let fallback = match language {
        Language::Chinese => chinese,
        Language::English => english,
    };

    dictionary()
        .strings
        .get(key)
        .and_then(|entry| entry.get(language))
        .map_or(Cow::Borrowed(fallback), Cow::Owned)
}

pub fn format_text(
    language: Language,
    key: &str,
    english: &'static str,
    chinese: &'static str,
    values: &[(&str, String)],
) -> String {
    let mut output = text(language, key, english, chinese).into_owned();
    for (name, value) in values {
        output = output.replace(&format!("{{{name}}}"), value);
    }
    output
}

fn dictionary() -> &'static LocalizationDocument {
    LOCALIZATION.get_or_init(
        || match load_localization(Path::new(DEFAULT_LOCALIZATION_PATH)) {
            Ok(document) => document,
            Err(error) => {
                eprintln!("localization load failed: {error:?}");
                LocalizationDocument::default()
            }
        },
    )
}

fn load_localization(path: &Path) -> anyhow::Result<LocalizationDocument> {
    let source = fs::read_to_string(path)?;
    Ok(ron::from_str(&source)?)
}

impl LocalizedString {
    fn get(&self, language: Language) -> Option<String> {
        let value = match language {
            Language::Chinese => self.chinese.trim(),
            Language::English => self.english.trim(),
        };
        (!value.is_empty()).then(|| value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_localization_document() {
        let document: LocalizationDocument = ron::from_str(
            r#"(
                strings: {
                    "menu.example": (english: "Example", chinese: "示例"),
                },
            )"#,
        )
        .expect("localization document should parse");

        assert_eq!(
            document.strings["menu.example"]
                .get(Language::Chinese)
                .as_deref(),
            Some("示例")
        );
    }

    #[test]
    fn missing_entry_uses_fallback() {
        assert_eq!(
            text(Language::English, "missing.key", "Fallback", "回退").as_ref(),
            "Fallback"
        );
    }

    #[test]
    fn bundled_localization_file_contains_menu_keys() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(DEFAULT_LOCALIZATION_PATH);
        let document = load_localization(&path).expect("bundled localization should load");

        assert!(document.strings.contains_key("main.title"));
        assert!(document.strings.contains_key("game.tab.quests.title"));
        assert!(document.strings.contains_key("hud.weather.clear"));
        assert!(document.strings.contains_key("notice.pickup"));
        assert!(
            document
                .strings
                .contains_key("activity.event.scan_recorded.title")
        );
        assert!(document.strings.contains_key("objective.status.active"));
        assert!(
            document
                .strings
                .contains_key("game.activity.category.objective")
        );
    }
}
