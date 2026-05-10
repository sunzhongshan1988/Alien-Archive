use eframe::egui::{self, Align2, Color32, FontId, Pos2, Response, Sense, vec2};

use crate::ui::theme::{THEME_ACCENT_DIM, THEME_MUTED_TEXT, THEME_PANEL_BG_SOFT, THEME_TEXT};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeBadge<'a> {
    pub(crate) label: &'a str,
    pub(crate) color: Color32,
}

pub(crate) fn tree_row<'a>(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    detail: &str,
    badges: impl IntoIterator<Item = TreeBadge<'a>>,
) -> Response {
    let row_height = if detail.is_empty() { 34.0 } else { 52.0 };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let background_rect = rect.shrink2(vec2(0.0, 3.0));
    if selected {
        ui.painter()
            .rect_filled(background_rect.expand(1.0), 2.0, THEME_ACCENT_DIM);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(background_rect.expand(1.0), 2.0, THEME_PANEL_BG_SOFT);
    }

    let content_rect = rect.shrink2(vec2(4.0, 5.0));
    let clip_rect = content_rect.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip_rect);
    let label_font = FontId::proportional(14.0);
    let detail_font = FontId::proportional(12.0);
    let label_y = if detail.is_empty() {
        content_rect.center().y
    } else {
        content_rect.top() + 11.0
    };
    let label_pos = Pos2::new(content_rect.left() + 2.0, label_y);
    painter.text(
        label_pos,
        Align2::LEFT_CENTER,
        label,
        label_font,
        THEME_TEXT,
    );

    let mut badge_right = content_rect.right() - 58.0;
    for badge in badges.into_iter().collect::<Vec<_>>().into_iter().rev() {
        let width = badge_width(badge.label);
        let badge_pos = Pos2::new(badge_right, label_y);
        painter.text(
            badge_pos,
            Align2::RIGHT_CENTER,
            badge.label,
            detail_font.clone(),
            badge.color,
        );
        badge_right -= width;
    }

    if !detail.is_empty() {
        painter.text(
            Pos2::new(content_rect.left() + 2.0, content_rect.bottom() - 10.0),
            Align2::LEFT_CENTER,
            detail,
            detail_font,
            THEME_MUTED_TEXT,
        );
    }

    response
}

fn badge_width(label: &str) -> f32 {
    label.chars().count() as f32 * 7.0 + 14.0
}
