use eframe::egui::{self, TextEdit};

pub(crate) const PROPERTY_LABEL_WIDTH: f32 = 82.0;

pub(crate) fn property_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.horizontal(|ui| {
        ui.add_sized(
            [PROPERTY_LABEL_WIDTH, ui.spacing().interact_size.y],
            egui::Label::new(label),
        );
        add_contents(ui)
    })
}

pub(crate) fn property_text_edit(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    property_row(ui, label, |ui| {
        ui.add(TextEdit::singleline(value).desired_width(ui.available_width()))
            .changed()
    })
    .inner
}

pub(crate) fn property_options(
    ui: &mut egui::Ui,
    label: &str,
    id_salt: impl std::hash::Hash,
    value: &mut String,
    options: &[String],
) -> bool {
    if options.is_empty() {
        return false;
    }

    let mut changed = false;
    property_row(ui, label, |ui| {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text("选择常用值")
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
