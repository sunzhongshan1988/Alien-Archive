use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use content::{CodexDatabase, CodexEntry};
use runtime::{Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::ui::menu_style::{color, inset_rect};
use crate::ui::menu_widgets::{
    contain_rect, draw_bar, draw_border, draw_inner_panel, draw_screen_rect, draw_text_strong,
    screen_rect,
};
use crate::ui::text::{TextSprite, draw_text, draw_text_centered, upload_text};

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

pub(super) fn draw_codex_entry_card(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    entry: &CodexEntryCardText,
    selected: bool,
    index: usize,
    scale: f32,
) {
    draw_inner_panel(renderer, viewport, card, scale);
    if selected {
        draw_border(
            renderer,
            viewport,
            card,
            2.0 * scale,
            Color::rgba(0.58, 0.98, 1.0, 0.96),
        );
    }

    let image_size = 58.0 * scale;
    let image_rect = Rect::new(
        Vec2::new(
            card.right() - image_size - 16.0 * scale,
            card.origin.y + 14.0 * scale,
        ),
        Vec2::new(image_size, image_size),
    );
    draw_codex_glyph(renderer, viewport, image_rect, index, scale);
    if !entry.unlocked {
        draw_screen_rect(
            renderer,
            viewport,
            image_rect,
            Color::rgba(0.0, 0.0, 0.0, 0.46),
        );
    }

    let text_x = card.origin.x + 18.0 * scale;
    let title_color = if entry.unlocked {
        color::TEXT_PRIMARY
    } else {
        color::TEXT_MUTED
    };
    draw_text_strong(
        renderer,
        &entry.title,
        viewport,
        text_x,
        card.origin.y + 10.0 * scale,
        title_color,
        scale,
    );
    draw_text(
        renderer,
        &entry.category,
        viewport,
        text_x,
        card.origin.y + 34.0 * scale,
        Color::rgba(0.46, 0.88, 0.96, 0.96),
    );
    draw_text(
        renderer,
        &entry.status,
        viewport,
        card.right() - image_size - 16.0 * scale,
        image_rect.bottom() + 4.0 * scale,
        if entry.unlocked {
            color::TEXT_GREEN
        } else {
            color::TEXT_DIM
        },
    );

    for (line_index, line) in entry.description_lines.iter().enumerate() {
        draw_text(
            renderer,
            line,
            viewport,
            text_x,
            card.origin.y + (56.0 + line_index as f32 * 17.0) * scale,
            if entry.unlocked {
                color::TEXT_SECONDARY
            } else {
                color::TEXT_DIM
            },
        );
    }
}

pub(super) fn draw_codex_discovery_card(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    index: usize,
    label: &TextSprite,
    value: &TextSprite,
    ratio: f32,
    scale: f32,
) {
    let content = inset_rect(card, 4.0 * scale);
    let image = Rect::new(
        content.origin,
        Vec2::new(content.size.x, content.size.y - 84.0 * scale),
    );
    let info_top = image.bottom() + 2.0 * scale;
    draw_codex_glyph(renderer, viewport, image, index, scale);
    draw_text_centered(
        renderer,
        label,
        viewport,
        content.origin.x + content.size.x * 0.5,
        info_top,
        color::TEXT_PRIMARY,
    );
    draw_text_centered(
        renderer,
        value,
        viewport,
        content.origin.x + content.size.x * 0.5,
        info_top + 25.0 * scale,
        color::TEXT_SECONDARY,
    );
    draw_bar(
        renderer,
        viewport,
        Rect::new(
            Vec2::new(content.origin.x + 10.0 * scale, info_top + 49.0 * scale),
            Vec2::new(content.size.x - 20.0 * scale, 6.0 * scale),
        ),
        ratio,
        scale,
    );
}

fn draw_codex_glyph(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    card: Rect,
    index: usize,
    scale: f32,
) {
    let texture_id = codex_thumbnail_texture_id(index);
    if let Some(image_size) = renderer.texture_size(texture_id) {
        let frame = inset_rect(card, 2.0 * scale);
        renderer.draw_image(
            texture_id,
            screen_rect(viewport, contain_rect(frame, image_size)),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        return;
    }

    let color = match index {
        0 => Color::rgba(0.56, 0.42, 0.95, 0.92),
        1 => Color::rgba(0.34, 0.88, 1.0, 0.92),
        2 => Color::rgba(0.58, 0.64, 1.0, 0.92),
        _ => Color::rgba(0.36, 0.92, 1.0, 0.92),
    };

    if card.size.y < 160.0 * scale && card.size.x > 240.0 * scale {
        let glyph = Rect::new(
            Vec2::new(card.origin.x + 28.0 * scale, card.origin.y + 34.0 * scale),
            Vec2::new(44.0 * scale, 44.0 * scale),
        );
        draw_screen_rect(
            renderer,
            viewport,
            glyph,
            Color::rgba(0.030, 0.070, 0.086, 0.90),
        );
        draw_border(renderer, viewport, glyph, 2.0 * scale, color);
        draw_screen_rect(renderer, viewport, inset_rect(glyph, 14.0 * scale), color);
        return;
    }

    let center = Vec2::new(
        card.origin.x + card.size.x * 0.5,
        card.origin.y + card.size.y * 0.5,
    );
    match index {
        0 => {
            let body = Rect::new(
                Vec2::new(center.x - 26.0 * scale, center.y - 20.0 * scale),
                Vec2::new(52.0 * scale, 34.0 * scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                body,
                Color::rgba(0.28, 0.19, 0.42, 0.95),
            );
            draw_border(renderer, viewport, body, 2.0 * scale, color);
            for eye in [-1.0, 1.0] {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(center.x + eye * 11.0 * scale, center.y - 8.0 * scale),
                        Vec2::new(6.0 * scale, 6.0 * scale),
                    ),
                    Color::rgba(0.70, 0.98, 1.0, 0.96),
                );
            }
            for leg in 0..4 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - 28.0 * scale + leg as f32 * 18.0 * scale,
                            center.y + 16.0 * scale,
                        ),
                        Vec2::new(7.0 * scale, 20.0 * scale),
                    ),
                    color,
                );
            }
        }
        1 => {
            for tier in 0..4 {
                let width = (64.0 - tier as f32 * 12.0) * scale;
                let height = (18.0 + tier as f32 * 8.0) * scale;
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y + 38.0 * scale - tier as f32 * 25.0 * scale,
                        ),
                        Vec2::new(width, height),
                    ),
                    Color::rgba(0.10, 0.32, 0.42, 0.92),
                );
                draw_border(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y + 38.0 * scale - tier as f32 * 25.0 * scale,
                        ),
                        Vec2::new(width, height),
                    ),
                    1.0 * scale,
                    color,
                );
            }
            draw_screen_rect(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(center.x - 5.0 * scale, center.y - 62.0 * scale),
                    Vec2::new(10.0 * scale, 28.0 * scale),
                ),
                color,
            );
        }
        2 => {
            for band in 0..5 {
                let width = (78.0 - (band as f32 - 2.0).abs() * 13.0) * scale;
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            center.x - width * 0.5,
                            center.y - 34.0 * scale + band as f32 * 14.0 * scale,
                        ),
                        Vec2::new(width, 10.0 * scale),
                    ),
                    if band == 2 {
                        Color::rgba(0.34, 0.88, 1.0, 0.92)
                    } else {
                        Color::rgba(0.24, 0.22, 0.48, 0.86)
                    },
                );
            }
            draw_border(
                renderer,
                viewport,
                Rect::new(
                    Vec2::new(center.x - 48.0 * scale, center.y - 42.0 * scale),
                    Vec2::new(96.0 * scale, 84.0 * scale),
                ),
                1.0 * scale,
                color,
            );
        }
        _ => {
            let tablet = Rect::new(
                Vec2::new(center.x - 34.0 * scale, center.y - 48.0 * scale),
                Vec2::new(68.0 * scale, 96.0 * scale),
            );
            draw_screen_rect(
                renderer,
                viewport,
                tablet,
                Color::rgba(0.06, 0.20, 0.28, 0.94),
            );
            draw_border(renderer, viewport, tablet, 2.0 * scale, color);
            for row in 0..4 {
                draw_screen_rect(
                    renderer,
                    viewport,
                    Rect::new(
                        Vec2::new(
                            tablet.origin.x + 14.0 * scale,
                            tablet.origin.y + (18.0 + row as f32 * 16.0) * scale,
                        ),
                        Vec2::new(40.0 * scale, 3.0 * scale),
                    ),
                    Color::rgba(0.55, 0.95, 1.0, 0.78),
                );
            }
        }
    }
}

fn codex_thumbnail_texture_id(index: usize) -> &'static str {
    match index {
        0 => "menu.codex_alien_life",
        1 => "menu.codex_relic_tech",
        2 => "menu.codex_star_geography",
        _ => "menu.codex_civilization",
    }
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

    #[test]
    fn thumbnail_texture_ids_keep_stable_fallback() {
        assert_eq!(codex_thumbnail_texture_id(0), "menu.codex_alien_life");
        assert_eq!(codex_thumbnail_texture_id(99), "menu.codex_civilization");
    }
}
