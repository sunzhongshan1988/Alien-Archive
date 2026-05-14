use eframe::egui::{self, RichText};

use crate::ui::{
    panel_surface::detail_surface,
    theme::{THEME_ACCENT_STRONG, THEME_ERROR, THEME_MUTED_TEXT, THEME_WARNING},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValidationLevel {
    Error,
    Warning,
}

pub(crate) struct ValidationMessage {
    pub(crate) level: ValidationLevel,
    pub(crate) message: String,
}

pub(crate) fn validation_panel(ui: &mut egui::Ui, title: &str, messages: &[ValidationMessage]) {
    detail_surface(ui, |ui| {
        ui.heading(title);
        ui.separator();
        if messages.is_empty() {
            ui.label(RichText::new("没有发现结构问题").color(THEME_ACCENT_STRONG));
            return;
        }
        let errors = messages
            .iter()
            .filter(|message| message.level == ValidationLevel::Error)
            .count();
        let warnings = messages
            .iter()
            .filter(|message| message.level == ValidationLevel::Warning)
            .count();
        ui.label(
            RichText::new(format!("{errors} 个错误 / {warnings} 个警告")).color(if errors > 0 {
                THEME_ERROR
            } else {
                THEME_WARNING
            }),
        );
        ui.add_space(4.0);
        for message in messages {
            let color = match message.level {
                ValidationLevel::Error => THEME_ERROR,
                ValidationLevel::Warning => THEME_WARNING,
            };
            ui.label(RichText::new(&message.message).color(color));
        }
    });
}

pub(crate) fn info_panel(ui: &mut egui::Ui, title: &str, lines: impl IntoIterator<Item = String>) {
    detail_surface(ui, |ui| {
        ui.heading(title);
        ui.separator();
        for line in lines {
            ui.label(RichText::new(line).color(THEME_MUTED_TEXT));
        }
    });
}
