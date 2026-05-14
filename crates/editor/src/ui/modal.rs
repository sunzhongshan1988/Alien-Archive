use eframe::egui::{self, Button, RichText, vec2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditorModalAction {
    None,
    Save,
    Apply,
    Cancel,
}

pub(crate) fn standard_modal(
    ctx: &egui::Context,
    id: &'static str,
    title: &str,
    width: f32,
    _min_height: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> EditorModalAction {
    let frame = egui::Frame::popup(ctx.global_style().as_ref())
        .inner_margin(egui::Margin::symmetric(14, 12));
    let response = egui::Modal::new(egui::Id::new(id))
        .frame(frame)
        .show(ctx, |ui| {
            let mut action = EditorModalAction::None;
            ui.set_width(width);

            ui.horizontal(|ui| {
                ui.heading(title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            vec2(28.0, 28.0),
                            Button::new(RichText::new("×").size(22.0)).frame(false),
                        )
                        .on_hover_text("关闭")
                        .clicked()
                    {
                        action = EditorModalAction::Cancel;
                    }
                });
            });
            ui.separator();

            add_contents(ui);

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("取消").clicked() {
                    action = EditorModalAction::Cancel;
                }
                if ui.button("应用").clicked() {
                    action = EditorModalAction::Apply;
                }
                if ui.button(RichText::new("保存").strong()).clicked() {
                    action = EditorModalAction::Save;
                }
            });

            action
        });

    if response.should_close() && response.inner == EditorModalAction::None {
        EditorModalAction::Cancel
    } else {
        response.inner
    }
}
