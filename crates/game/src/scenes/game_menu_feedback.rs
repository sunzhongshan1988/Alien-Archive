use std::path::Path;

use runtime::{Color, Rect, Renderer, Vec2};

use crate::ui::{
    menu_style::MenuLayout,
    menu_widgets::{draw_border, draw_screen_rect},
    text::{TextSprite, draw_text_centered},
};

use super::Language;

const MENU_TOAST_VISIBLE_TIME: f32 = 2.8;
const MENU_TOAST_WIDTH: f32 = 560.0;
const MENU_TOAST_HEIGHT: f32 = 42.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum GameMenuToastTone {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub(super) struct GameMenuToast {
    pub message: String,
    pub tone: GameMenuToastTone,
    pub remaining: f32,
}

impl GameMenuToast {
    pub(super) fn new(message: impl Into<String>, tone: GameMenuToastTone) -> Self {
        Self {
            message: message.into(),
            tone,
            remaining: MENU_TOAST_VISIBLE_TIME,
        }
    }
}

pub(super) fn draw_game_menu_toast(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    layout: &MenuLayout,
    toast: &GameMenuToast,
    text: &TextSprite,
) {
    let rect = game_menu_toast_rect(layout);
    draw_screen_rect(renderer, viewport, rect, game_menu_toast_fill(toast.tone));
    draw_border(
        renderer,
        viewport,
        rect,
        1.0 * layout.scale,
        game_menu_toast_border(toast.tone),
    );
    draw_text_centered(
        renderer,
        text,
        viewport,
        rect.origin.x + rect.size.x * 0.5,
        centered_text_y(rect, text, 0.0),
        game_menu_toast_text(toast.tone),
    );
}

pub(super) fn game_menu_log_jump_message(language: Language) -> &'static str {
    match language {
        Language::Chinese => "已切到外勤日志",
        Language::English => "Field log opened",
    }
}

pub(super) fn game_menu_action_pending_message(language: Language) -> &'static str {
    match language {
        Language::Chinese => "这个动作还在接入中",
        Language::English => "This action is still being wired in",
    }
}

pub(super) fn game_menu_save_success_message(language: Language, save_path: &Path) -> String {
    let target = save_target_label(language, save_path);
    match language {
        Language::Chinese => format!("已手动保存到 {target}"),
        Language::English => format!("Saved manually to {target}"),
    }
}

pub(super) fn game_menu_save_error_message(language: Language, detail: &str) -> String {
    match language {
        Language::Chinese => format!("手动保存失败：{}", short_menu_error_detail(detail)),
        Language::English => format!("Manual save failed: {}", short_menu_error_detail(detail)),
    }
}

fn game_menu_toast_rect(layout: &MenuLayout) -> Rect {
    let width = (MENU_TOAST_WIDTH * layout.scale).min(layout.root.size.x - 48.0 * layout.scale);
    Rect::new(
        Vec2::new(
            layout.root.origin.x + (layout.root.size.x - width) * 0.5,
            layout.bottom.origin.y - (MENU_TOAST_HEIGHT + 12.0) * layout.scale,
        ),
        Vec2::new(width, MENU_TOAST_HEIGHT * layout.scale),
    )
}

fn game_menu_toast_fill(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.015, 0.052, 0.066, 0.94),
        GameMenuToastTone::Success => Color::rgba(0.018, 0.090, 0.062, 0.95),
        GameMenuToastTone::Error => Color::rgba(0.126, 0.032, 0.034, 0.95),
    }
}

fn game_menu_toast_border(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.34, 0.90, 1.0, 0.88),
        GameMenuToastTone::Success => Color::rgba(0.42, 1.0, 0.72, 0.90),
        GameMenuToastTone::Error => Color::rgba(1.0, 0.42, 0.42, 0.92),
    }
}

fn game_menu_toast_text(tone: GameMenuToastTone) -> Color {
    match tone {
        GameMenuToastTone::Info => Color::rgba(0.82, 0.96, 1.0, 1.0),
        GameMenuToastTone::Success => Color::rgba(0.82, 1.0, 0.90, 1.0),
        GameMenuToastTone::Error => Color::rgba(1.0, 0.82, 0.82, 1.0),
    }
}

fn centered_text_y(rect: Rect, text: &TextSprite, y_offset: f32) -> f32 {
    let text_padding = 8.0;
    let visual_height = (text.size.y - text_padding * 2.0).max(0.0);
    rect.origin.y + (rect.size.y - visual_height) * 0.5 + y_offset - text_padding
}

fn save_target_label(language: Language, save_path: &Path) -> String {
    if let Some(slot_index) = save_slot_index_from_path(save_path) {
        return match language {
            Language::Chinese => format!("槽位 {}", slot_index + 1),
            Language::English => format!("Slot {}", slot_index + 1),
        };
    }

    save_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("save")
        .to_owned()
}

fn save_slot_index_from_path(save_path: &Path) -> Option<usize> {
    let stem = save_path.file_stem()?.to_str()?;
    let number = stem.strip_prefix("profile_")?.parse::<usize>().ok()?;
    number.checked_sub(1)
}

fn short_menu_error_detail(detail: &str) -> &str {
    const MAX_CHARS: usize = 42;
    let trimmed = detail.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed;
    }

    let cut = trimmed
        .char_indices()
        .nth(MAX_CHARS)
        .map_or(trimmed.len(), |(index, _)| index);
    &trimmed[..cut]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_feedback_names_fixed_slots() {
        assert_eq!(
            save_target_label(Language::Chinese, Path::new("saves/profile_02.ron")),
            "槽位 2"
        );
        assert!(
            game_menu_save_success_message(Language::English, Path::new("saves/profile_03.ron"))
                .contains("Slot 3")
        );
        assert!(game_menu_save_error_message(Language::Chinese, "disk is full").contains("失败"));
    }

    #[test]
    fn save_error_detail_is_trimmed_without_splitting_utf8() {
        let detail = " 这是一个非常非常非常非常非常非常非常非常非常长的错误消息 ";

        assert!(short_menu_error_detail(detail).chars().count() <= 42);
        assert!(game_menu_save_error_message(Language::Chinese, detail).contains("失败"));
    }
}
