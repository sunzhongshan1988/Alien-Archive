use eframe::egui::{self, TextEdit};

use crate::ui::theme::THEME_MUTED_TEXT;

pub(crate) const GRID_LABEL_WIDTH: f32 = 118.0;

pub(crate) fn property_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal(|ui| {
        ui.add_sized(
            [GRID_LABEL_WIDTH, ui.spacing().interact_size.y],
            egui::Label::new(label),
        );
        add_contents(ui)
    })
}

pub(crate) fn text_field(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    value: &mut String,
) -> bool {
    property_row(ui, label, |ui| {
        ui.add(
            TextEdit::singleline(value)
                .id_salt(id)
                .desired_width(ui.available_width()),
        )
        .changed()
    })
    .inner
}

pub(crate) fn multiline_field(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    value: &mut String,
    rows: usize,
) -> bool {
    property_row(ui, label, |ui| {
        ui.add(
            TextEdit::multiline(value)
                .id_salt(id)
                .desired_width(ui.available_width())
                .desired_rows(rows),
        )
        .changed()
    })
    .inner
}

pub(crate) fn picker_field(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    value: &mut String,
    placeholder: &str,
    options: &[String],
) -> bool {
    let mut changed = text_field(ui, label, (&id, "text"), value);
    if options.is_empty() {
        return changed;
    }
    property_row(ui, "常用", |ui| {
        egui::ComboBox::from_id_salt((&id, "combo"))
            .selected_text(if value.trim().is_empty() {
                placeholder
            } else {
                value.as_str()
            })
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for option in options {
                    if ui
                        .selectable_label(value == option, option.as_str())
                        .clicked()
                    {
                        *value = option.clone();
                        changed = true;
                        ui.close();
                    }
                }
            });
    });
    changed
}

pub(crate) fn helper_text(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(THEME_MUTED_TEXT));
}
