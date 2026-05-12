use std::ops::Range;

use anyhow::Result;
use runtime::{Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::save::ActivityLogEntrySave;
use crate::ui::{
    game_menu_content::activity_category_label,
    menu_style::MenuLayout,
    menu_widgets::draw_screen_rect,
    text::{TextSprite, upload_text},
};

use super::Language;

pub(super) const ACTIVITY_LOG_ROW_HEIGHT: f32 = 55.0;
pub(super) const ACTIVITY_LOG_ROW_GAP: f32 = 8.0;
pub(super) const ACTIVITY_LOG_HEADER_HEIGHT: f32 = 62.0;
pub(super) const ACTIVITY_LOG_BOTTOM_PADDING: f32 = 16.0;

pub(super) struct ActivityLogRowText {
    pub category: TextSprite,
    pub title: TextSprite,
    pub detail: TextSprite,
    pub meta: TextSprite,
    pub category_key: String,
}

pub(super) fn upload_activity_log_rows(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    entries: &[ActivityLogEntrySave],
    language: Language,
) -> Result<Vec<ActivityLogRowText>> {
    entries
        .iter()
        .rev()
        .enumerate()
        .map(|(index, entry)| {
            Ok(ActivityLogRowText {
                category: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_activity_category_{index}"),
                    activity_category_label(&entry.category, language).as_ref(),
                    14.0,
                )?,
                title: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_activity_title_{index}"),
                    &short_activity_text(&entry.title, 28),
                    17.0,
                )?,
                detail: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_activity_detail_{index}"),
                    &short_activity_text(&entry.detail, 58),
                    14.0,
                )?,
                meta: upload_text(
                    renderer,
                    font,
                    &format!("game_menu_activity_meta_{index}"),
                    &activity_log_meta(entry, language),
                    13.0,
                )?,
                category_key: entry.category.clone(),
            })
        })
        .collect()
}

pub(super) fn short_activity_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        return trimmed.to_owned();
    }

    let mut shortened = trimmed
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

pub(super) fn activity_log_meta(entry: &ActivityLogEntrySave, language: Language) -> String {
    let scene = if entry.scene.trim().is_empty() {
        "-"
    } else {
        entry.scene.as_str()
    };
    let prefix = match language {
        Language::Chinese => "记录",
        Language::English => "Log",
    };
    format!("{prefix} #{:03} · {scene}", entry.sequence)
}

pub(super) fn activity_category_color(category: &str) -> Color {
    match category {
        "pickup" => Color::rgba(0.06, 0.34, 0.22, 0.88),
        "scan" => Color::rgba(0.07, 0.28, 0.42, 0.88),
        "unlock" => Color::rgba(0.30, 0.20, 0.48, 0.88),
        "status" => Color::rgba(0.42, 0.24, 0.08, 0.88),
        "objective" => Color::rgba(0.04, 0.36, 0.34, 0.88),
        _ => Color::rgba(0.16, 0.24, 0.28, 0.88),
    }
}

pub(super) fn activity_objective_panel_rect(layout: &MenuLayout) -> Rect {
    let content = layout.content_body();
    Rect::new(
        content.origin,
        Vec2::new(content.size.x, 166.0 * layout.scale),
    )
}

pub(super) fn activity_log_panel_rect(layout: &MenuLayout) -> Rect {
    let objective_panel = activity_objective_panel_rect(layout);
    let top = objective_panel.bottom() + 18.0 * layout.scale;
    let bottom = layout.bottom.origin.y - 12.0 * layout.scale;
    Rect::new(
        Vec2::new(layout.content_body().origin.x, top),
        Vec2::new(layout.content_body().size.x, (bottom - top).max(0.0)),
    )
}

pub(super) fn activity_log_row_area_rect(log_panel: Rect, scale: f32) -> Rect {
    let top = log_panel.origin.y + ACTIVITY_LOG_HEADER_HEIGHT * scale;
    let bottom = log_panel.bottom() - ACTIVITY_LOG_BOTTOM_PADDING * scale;
    Rect::new(
        Vec2::new(log_panel.origin.x + 18.0 * scale, top),
        Vec2::new(
            (log_panel.size.x - 36.0 * scale).max(0.0),
            (bottom - top).max(0.0),
        ),
    )
}

pub(super) fn activity_log_visible_capacity(log_panel: Rect, scale: f32) -> usize {
    let rows = activity_log_row_area_rect(log_panel, scale);
    let row_height = ACTIVITY_LOG_ROW_HEIGHT * scale;
    let row_gap = ACTIVITY_LOG_ROW_GAP * scale;
    if rows.size.y < row_height {
        return 1;
    }

    ((rows.size.y + row_gap) / (row_height + row_gap))
        .floor()
        .max(1.0) as usize
}

pub(super) fn activity_log_visible_range(
    total_rows: usize,
    scroll: usize,
    visible_rows: usize,
) -> Range<usize> {
    let visible_rows = visible_rows.max(1);
    let start = scroll.min(total_rows.saturating_sub(visible_rows));
    let end = (start + visible_rows).min(total_rows);
    start..end
}

pub(super) fn activity_log_max_scroll(total_rows: usize, visible_rows: usize) -> usize {
    total_rows.saturating_sub(visible_rows.max(1))
}

pub(super) fn activity_log_clamped_scroll(scroll: usize, total_rows: usize) -> usize {
    scroll.min(total_rows.saturating_sub(1))
}

pub(super) fn activity_log_scrolled(
    scroll: usize,
    rows: isize,
    total_rows: usize,
    visible_rows: usize,
) -> usize {
    if rows == 0 {
        return scroll;
    }

    let current = scroll as isize;
    let max_scroll = activity_log_max_scroll(total_rows, visible_rows) as isize;
    (current + rows).clamp(0, max_scroll) as usize
}

pub(super) fn activity_log_scroll_from_track(
    cursor_y: f32,
    layout: &MenuLayout,
    total_rows: usize,
) -> usize {
    let log_panel = activity_log_panel_rect(layout);
    let visible_rows = activity_log_visible_capacity(log_panel, layout.scale);
    let max_scroll = activity_log_max_scroll(total_rows, visible_rows);
    if max_scroll == 0 {
        return 0;
    }

    let track = activity_log_scrollbar_track_rect(log_panel, layout.scale);
    let ratio = ((cursor_y - track.origin.y) / track.size.y).clamp(0.0, 1.0);
    (ratio * max_scroll as f32).round() as usize
}

pub(super) fn activity_log_scrollbar_track_rect(log_panel: Rect, scale: f32) -> Rect {
    Rect::new(
        Vec2::new(
            log_panel.right() - 18.0 * scale,
            log_panel.origin.y + 58.0 * scale,
        ),
        Vec2::new(
            4.0 * scale,
            (log_panel.size.y - 76.0 * scale).max(24.0 * scale),
        ),
    )
}

pub(super) fn draw_activity_log_scrollbar(
    renderer: &mut dyn Renderer,
    viewport: Vec2,
    log_panel: Rect,
    total_rows: usize,
    scroll: usize,
    visible_rows: usize,
    scale: f32,
) {
    let track = activity_log_scrollbar_track_rect(log_panel, scale);
    draw_screen_rect(
        renderer,
        viewport,
        track,
        Color::rgba(0.035, 0.082, 0.096, 0.86),
    );

    let visible_ratio = (visible_rows.max(1) as f32 / total_rows.max(1) as f32).min(1.0);
    let thumb_height = (track.size.y * visible_ratio).max(24.0 * scale);
    let max_scroll = total_rows.saturating_sub(visible_rows.max(1));
    let thumb_travel = (track.size.y - thumb_height).max(0.0);
    let scroll_ratio = if max_scroll == 0 {
        0.0
    } else {
        scroll.min(max_scroll) as f32 / max_scroll as f32
    };
    let thumb = Rect::new(
        Vec2::new(track.origin.x, track.origin.y + thumb_travel * scroll_ratio),
        Vec2::new(track.size.x, thumb_height),
    );
    draw_screen_rect(
        renderer,
        viewport,
        thumb,
        Color::rgba(0.40, 0.94, 1.0, 0.92),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_log_visible_range_clamps_to_scrollable_window() {
        assert_eq!(activity_log_visible_range(3, 0, 5), 0..3);
        assert_eq!(activity_log_visible_range(12, 0, 3), 0..3);
        assert_eq!(activity_log_visible_range(12, 4, 3), 4..7);
        assert_eq!(activity_log_visible_range(12, 99, 3), 9..12);
    }

    #[test]
    fn activity_log_capacity_fits_only_whole_rows_inside_panel() {
        let panel = Rect::new(Vec2::ZERO, Vec2::new(500.0, 235.0));

        assert_eq!(activity_log_visible_capacity(panel, 1.0), 2);
    }

    #[test]
    fn activity_log_rows_stay_inside_frame_at_screenshot_size() {
        let layout = MenuLayout::new(Vec2::new(1534.0, 800.0));
        let log_panel = activity_log_panel_rect(&layout);
        let row_area = activity_log_row_area_rect(log_panel, layout.scale);
        let visible_rows = activity_log_visible_capacity(log_panel, layout.scale);
        let row_height = ACTIVITY_LOG_ROW_HEIGHT * layout.scale;
        let row_gap = ACTIVITY_LOG_ROW_GAP * layout.scale;
        let last_row_bottom = row_area.origin.y
            + visible_rows as f32 * row_height
            + visible_rows.saturating_sub(1) as f32 * row_gap;

        assert!(visible_rows > 0);
        assert!(last_row_bottom <= log_panel.bottom() - ACTIVITY_LOG_BOTTOM_PADDING * layout.scale);
        assert!(log_panel.bottom() < layout.bottom.origin.y);
    }

    #[test]
    fn activity_log_scroll_math_clamps_to_available_rows() {
        assert_eq!(activity_log_max_scroll(12, 3), 9);
        assert_eq!(activity_log_clamped_scroll(99, 0), 0);
        assert_eq!(activity_log_scrolled(4, 10, 12, 3), 9);
        assert_eq!(activity_log_scrolled(4, -10, 12, 3), 0);
    }
}
