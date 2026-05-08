use eframe::egui::{self, Sense, Stroke, StrokeKind, Vec2, vec2};

use crate::ui::theme::{
    THEME_ACCENT_STRONG, THEME_APP_BG, THEME_BORDER, THEME_MUTED_TEXT, THEME_PANEL_BG,
    THEME_PANEL_BG_SOFT, THEME_TEXT,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorTabs {
    height: f32,
    min_tab_width: f32,
    max_tab_width: f32,
    trailing_width: f32,
}

impl Default for EditorTabs {
    fn default() -> Self {
        Self {
            height: 30.0,
            min_tab_width: 68.0,
            max_tab_width: 104.0,
            trailing_width: 30.0,
        }
    }
}

impl EditorTabs {
    pub(crate) fn show<T>(
        self,
        ui: &mut egui::Ui,
        active: &mut T,
        tabs: impl IntoIterator<Item = T>,
        label_for: impl Fn(T) -> &'static str,
    ) -> egui::InnerResponse<()>
    where
        T: Copy + Eq,
    {
        let tabs = tabs.into_iter().collect::<Vec<_>>();
        let tab_width = ((ui.available_width() - self.trailing_width) / tabs.len().max(1) as f32)
            .clamp(self.min_tab_width, self.max_tab_width);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for tab in tabs {
                let selected = *active == tab;
                let label = label_for(tab);
                let response = draw_tab(ui, Vec2::new(tab_width, self.height), label, selected);
                if response.clicked() {
                    *active = tab;
                }
            }
        })
    }
}

fn draw_tab(ui: &mut egui::Ui, size: Vec2, label: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if selected {
        THEME_PANEL_BG
    } else if response.hovered() {
        THEME_PANEL_BG_SOFT
    } else {
        THEME_APP_BG
    };

    ui.painter().rect_filled(rect, 0.0, fill);
    if selected {
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, THEME_BORDER),
            StrokeKind::Inside,
        );
    }
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(
            if selected { 2.0 } else { 1.0 },
            if selected {
                THEME_ACCENT_STRONG
            } else {
                THEME_BORDER
            },
        ),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        if selected {
            THEME_TEXT
        } else {
            THEME_MUTED_TEXT
        },
    );

    response
}
