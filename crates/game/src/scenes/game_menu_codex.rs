use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use content::{CodexDatabase, CodexEntry};
use runtime::Renderer;
use rusttype::Font;

use crate::ui::text::{TextSprite, upload_text};

use super::Language;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct CodexMenuSnapshot {
    pub entries: Vec<CodexEntryView>,
}

impl CodexMenuSnapshot {
    pub(super) fn from_database(database: &CodexDatabase, scanned_ids: &HashSet<String>) -> Self {
        Self {
            entries: database
                .entries()
                .iter()
                .map(|entry| CodexEntryView::from_entry(entry, scanned_ids.contains(&entry.id)))
                .collect(),
        }
    }

    pub(super) fn unlocked_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.unlocked).count()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CodexEntryView {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub unlocked: bool,
}

impl CodexEntryView {
    fn from_entry(entry: &CodexEntry, unlocked: bool) -> Self {
        Self {
            id: entry.id.clone(),
            title: non_empty_or(&entry.title, &entry.id).to_owned(),
            category: non_empty_or(&entry.category, "Unknown").to_owned(),
            description: entry.description.clone(),
            unlocked,
        }
    }
}

pub(super) struct CodexSummaryText {
    pub label: TextSprite,
    pub value: TextSprite,
    pub ratio: f32,
}

pub(super) struct CodexEntryCardText {
    pub title: TextSprite,
    pub category: TextSprite,
    pub status: TextSprite,
    pub description_lines: Vec<TextSprite>,
    pub unlocked: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CodexSummaryView {
    pub label: String,
    pub value: String,
    pub ratio: f32,
}

pub(super) fn codex_summary_views(
    snapshot: &CodexMenuSnapshot,
    language: Language,
) -> Vec<CodexSummaryView> {
    let mut by_category = BTreeMap::<String, (usize, usize)>::new();
    for entry in &snapshot.entries {
        let counts = by_category.entry(entry.category.clone()).or_default();
        counts.1 += 1;
        if entry.unlocked {
            counts.0 += 1;
        }
    }

    let mut summaries = by_category
        .into_iter()
        .map(|(category, (unlocked, total))| CodexSummaryView {
            label: category,
            value: format!("{unlocked} / {total}"),
            ratio: unlocked as f32 / total.max(1) as f32,
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| right.ratio.total_cmp(&left.ratio));
    summaries.truncate(4);

    if summaries.is_empty() {
        summaries.push(CodexSummaryView {
            label: match language {
                Language::Chinese => "图鉴数据库".to_owned(),
                Language::English => "Codex Database".to_owned(),
            },
            value: "0 / 0".to_owned(),
            ratio: 0.0,
        });
    }

    summaries
}

pub(super) fn upload_codex_entry_cards(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    language: Language,
    snapshot: &CodexMenuSnapshot,
) -> Result<Vec<CodexEntryCardText>> {
    snapshot
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let title = if entry.unlocked {
                entry.title.as_str()
            } else {
                locked_codex_title(language)
            };
            let description = if entry.unlocked {
                non_empty_or(&entry.description, codex_empty_description(language))
            } else {
                locked_codex_description(language)
            };
            let description_lines = wrap_text(description, 54, 2);

            Ok(CodexEntryCardText {
                title: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_title_{index}"),
                    title,
                    19.0,
                )?,
                category: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_category_{index}"),
                    &entry.category,
                    14.0,
                )?,
                status: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_codex_entry_status_{index}"),
                    codex_status_label(language, entry.unlocked),
                    14.0,
                )?,
                description_lines: description_lines
                    .iter()
                    .enumerate()
                    .map(|(line_index, line)| {
                        upload_text(
                            renderer,
                            font,
                            &format!("game_menu_codex_entry_desc_{index}_{line_index}"),
                            line,
                            13.0,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?,
                unlocked: entry.unlocked,
            })
        })
        .collect()
}

fn locked_codex_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "未识别条目",
        Language::English => "Undiscovered Entry",
    }
}

fn codex_status_label(language: Language, unlocked: bool) -> &'static str {
    match (language, unlocked) {
        (Language::Chinese, true) => "已解锁",
        (Language::Chinese, false) => "未扫描",
        (Language::English, true) => "Unlocked",
        (Language::English, false) => "Locked",
    }
}

fn locked_codex_description(language: Language) -> &'static str {
    match language {
        Language::Chinese => "靠近目标并完成扫描后显示完整记录。",
        Language::English => "Scan the target to unlock the full field record.",
    }
}

fn codex_empty_description(language: Language) -> &'static str {
    match language {
        Language::Chinese => "该条目还没有正文记录。",
        Language::English => "No field note has been written for this entry.",
    }
}

fn wrap_text(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let next_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if next_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
            if lines.len() == max_lines {
                break;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn non_empty_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let value = value.trim();
    if value.is_empty() { fallback } else { value }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use content::CodexEntry;

    use super::*;

    #[test]
    fn snapshot_marks_scanned_entries_unlocked() {
        let mut database = CodexDatabase::new("Overworld");
        database.entries.push(CodexEntry {
            id: "codex.test.flora".to_owned(),
            category: "Flora".to_owned(),
            title: "Test Flora".to_owned(),
            description: "A plant used by the menu tests.".to_owned(),
            scan_time: Some(1.25),
            unlock_tags: vec!["flora".to_owned()],
            image: None,
        });
        database.entries.push(CodexEntry {
            id: "codex.test.ruin".to_owned(),
            category: "Ruins".to_owned(),
            title: "Test Ruin".to_owned(),
            description: "A ruin used by the menu tests.".to_owned(),
            scan_time: Some(1.25),
            unlock_tags: vec!["ruin".to_owned()],
            image: None,
        });
        database.reindex();

        let scanned_ids = HashSet::from(["codex.test.flora".to_owned()]);
        let snapshot = CodexMenuSnapshot::from_database(&database, &scanned_ids);

        assert_eq!(snapshot.unlocked_count(), 1);
        assert_eq!(snapshot.entries.len(), 2);
        assert!(snapshot.entries[0].unlocked);
        assert!(!snapshot.entries[1].unlocked);
    }

    #[test]
    fn summary_views_group_by_category_and_include_empty_fallback() {
        let snapshot = CodexMenuSnapshot {
            entries: vec![
                CodexEntryView {
                    id: "a".to_owned(),
                    title: "A".to_owned(),
                    category: "Flora".to_owned(),
                    description: String::new(),
                    unlocked: true,
                },
                CodexEntryView {
                    id: "b".to_owned(),
                    title: "B".to_owned(),
                    category: "Flora".to_owned(),
                    description: String::new(),
                    unlocked: false,
                },
            ],
        };

        let summaries = codex_summary_views(&snapshot, Language::English);

        assert_eq!(summaries[0].label, "Flora");
        assert_eq!(summaries[0].value, "1 / 2");
        assert_eq!(summaries[0].ratio, 0.5);

        let empty = codex_summary_views(&CodexMenuSnapshot::default(), Language::English);
        assert_eq!(empty[0].label, "Codex Database");
        assert_eq!(empty[0].value, "0 / 0");
    }

    #[test]
    fn wrap_text_respects_line_limit() {
        let lines = wrap_text(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda",
            18,
            2,
        );

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.len() <= 18));
    }
}
