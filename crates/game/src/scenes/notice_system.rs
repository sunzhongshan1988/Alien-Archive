use anyhow::Result;
use content::CodexDatabase;
use runtime::{Camera2d, Color, Rect, Renderer, Vec2};
use rusttype::Font;

use crate::ui::{
    menu_widgets::{draw_border, draw_screen_rect},
    text::{TextSprite, draw_text_centered, load_ui_font, upload_text},
};
use crate::world::MapUnlockRule;

use super::{Language, QuickItemUseResult, inventory_scene, profile_meter_label};

const NOTICE_TIME: f32 = 2.35;
const NOTICE_WIDTH_MIN: f32 = 300.0;
const NOTICE_WIDTH_MAX: f32 = 720.0;
const NOTICE_HEIGHT: f32 = 52.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoticeTone {
    Info,
    Success,
    Warning,
}

#[derive(Default)]
pub(super) struct NoticeState {
    message: String,
    tone: Option<NoticeTone>,
    timer: f32,
    font: Option<Font<'static>>,
    text_key: Option<String>,
    text: Option<TextSprite>,
}

impl NoticeState {
    pub(super) fn update(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
    }

    pub(super) fn push_pickup(&mut self, language: Language, item_id: &str, quantity: u32) {
        let item_name = inventory_scene::inventory_item_name(item_id, language);
        self.push(
            pickup_message(language, &item_name, quantity),
            NoticeTone::Success,
        );
    }

    pub(super) fn push_inventory_full(&mut self, language: Language) {
        self.push(inventory_full_message(language), NoticeTone::Warning);
    }

    pub(super) fn push_stamina_low(&mut self, language: Language) {
        self.push(stamina_low_message(language), NoticeTone::Warning);
    }

    pub(super) fn push_quick_item_use_result(
        &mut self,
        language: Language,
        result: &QuickItemUseResult,
    ) {
        match result {
            QuickItemUseResult::Empty => {
                self.push(quick_item_empty_message(language), NoticeTone::Warning);
            }
            QuickItemUseResult::NotUsable { item_id } => {
                let item_name = inventory_scene::inventory_item_name(item_id, language);
                self.push(
                    quick_item_not_usable_message(language, &item_name),
                    NoticeTone::Warning,
                );
            }
            QuickItemUseResult::AlreadyFull { item_id, meter_id } => {
                let item_name = inventory_scene::inventory_item_name(item_id, language);
                let meter_name = profile_meter_label(meter_id, language);
                self.push(
                    quick_item_full_message(language, &item_name, meter_name),
                    NoticeTone::Info,
                );
            }
            QuickItemUseResult::Used {
                item_id,
                meter_id,
                amount,
            } => {
                let item_name = inventory_scene::inventory_item_name(item_id, language);
                let meter_name = profile_meter_label(meter_id, language);
                self.push(
                    quick_item_used_message(language, &item_name, meter_name, *amount),
                    NoticeTone::Success,
                );
            }
        }
    }

    pub(super) fn push_locked_unlock_rule(
        &mut self,
        language: Language,
        unlock: Option<&MapUnlockRule>,
        database: &CodexDatabase,
    ) {
        let Some(unlock) = unlock else {
            self.push(generic_locked_message(language), NoticeTone::Warning);
            return;
        };

        if let Some(message) = unlock.locked_message.as_deref() {
            self.push(message.to_owned(), NoticeTone::Warning);
            return;
        }

        let codex_title = unlock
            .requires_codex_id
            .as_deref()
            .and_then(|id| database.get(id).map(|entry| entry.title.trim()))
            .filter(|title| !title.is_empty());
        let item_name = unlock
            .requires_item_id
            .as_deref()
            .map(|id| inventory_scene::inventory_item_name(id, language));
        self.push(
            locked_rule_message(language, codex_title, item_name.as_deref()),
            NoticeTone::Warning,
        );
    }

    pub(super) fn push_scan_complete(
        &mut self,
        language: Language,
        codex_id: &str,
        database: &CodexDatabase,
    ) {
        let title = database
            .get(codex_id)
            .map(|entry| entry.title.trim())
            .filter(|title| !title.is_empty())
            .unwrap_or(codex_id);
        self.push(scan_complete_message(language, title), NoticeTone::Info);
    }

    pub(super) fn draw(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let Some(tone) = self.tone else {
            return Ok(());
        };
        if self.timer <= 0.0 {
            return Ok(());
        }

        self.upload_textures_if_needed(renderer)?;
        let Some(text) = &self.text else {
            return Ok(());
        };

        let viewport = renderer.screen_size();
        renderer.set_camera(Camera2d::default());

        let alpha = (self.timer / NOTICE_TIME).clamp(0.0, 1.0).min(1.0);
        let width = (text.size.x + 72.0)
            .clamp(NOTICE_WIDTH_MIN, NOTICE_WIDTH_MAX)
            .min(viewport.x - 48.0);
        let rect = Rect::new(
            Vec2::new((viewport.x - width) * 0.5, 72.0),
            Vec2::new(width, NOTICE_HEIGHT),
        );
        let (fill, border, text_color) = notice_colors(tone, alpha);

        draw_screen_rect(renderer, viewport, rect, fill);
        draw_border(renderer, viewport, rect, 1.0, border);
        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(rect.origin, Vec2::new(4.0, rect.size.y)),
            Color::rgba(border.r, border.g, border.b, alpha),
        );
        draw_text_centered(
            renderer,
            text,
            viewport,
            rect.origin.x + rect.size.x * 0.5,
            rect.origin.y + (rect.size.y - text.size.y) * 0.5,
            text_color,
        );

        Ok(())
    }

    fn push(&mut self, message: String, tone: NoticeTone) {
        self.message = message;
        self.tone = Some(tone);
        self.timer = NOTICE_TIME;
        self.text_key = None;
    }

    fn upload_textures_if_needed(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let key = format!("{:?}|{}", self.tone, self.message);
        if self.text_key.as_deref() == Some(key.as_str()) {
            return Ok(());
        }

        if self.font.is_none() {
            self.font = Some(load_ui_font()?);
        }
        let font = self.font.as_ref().expect("notice font should be loaded");
        self.text = Some(upload_text(
            renderer,
            font,
            "interaction_notice_text",
            &self.message,
            18.0,
        )?);
        self.text_key = Some(key);
        Ok(())
    }
}

fn pickup_message(language: Language, item_name: &str, quantity: u32) -> String {
    match language {
        Language::Chinese => format!("获得 {item_name} x{quantity}"),
        Language::English => format!("Acquired {item_name} x{quantity}"),
    }
}

fn inventory_full_message(language: Language) -> String {
    match language {
        Language::Chinese => "背包已满，无法收集".to_owned(),
        Language::English => "Inventory full".to_owned(),
    }
}

fn stamina_low_message(language: Language) -> String {
    match language {
        Language::Chinese => "体力不足，无法跳跃".to_owned(),
        Language::English => "Stamina too low to jump".to_owned(),
    }
}

fn quick_item_empty_message(language: Language) -> String {
    match language {
        Language::Chinese => "当前快捷槽为空".to_owned(),
        Language::English => "Selected quick slot is empty".to_owned(),
    }
}

fn quick_item_not_usable_message(language: Language, item_name: &str) -> String {
    match language {
        Language::Chinese => format!("{item_name} 不能直接使用"),
        Language::English => format!("{item_name} cannot be used directly"),
    }
}

fn quick_item_full_message(language: Language, item_name: &str, meter_name: &str) -> String {
    match language {
        Language::Chinese => format!("{meter_name} 已满，无需使用 {item_name}"),
        Language::English => format!("{meter_name} is full; {item_name} not used"),
    }
}

fn quick_item_used_message(
    language: Language,
    item_name: &str,
    meter_name: &str,
    amount: u32,
) -> String {
    match language {
        Language::Chinese => format!("使用 {item_name}，{meter_name} +{amount}"),
        Language::English => format!("Used {item_name}: {meter_name} +{amount}"),
    }
}

fn generic_locked_message(language: Language) -> String {
    match language {
        Language::Chinese => "通路尚未解锁".to_owned(),
        Language::English => "Path locked".to_owned(),
    }
}

fn locked_rule_message(
    language: Language,
    codex_title: Option<&str>,
    item_name: Option<&str>,
) -> String {
    match (language, codex_title, item_name) {
        (Language::Chinese, Some(title), Some(item)) => {
            format!("需要扫描：{title}，并持有：{item}")
        }
        (Language::Chinese, Some(title), None) => format!("需要先扫描：{title}"),
        (Language::Chinese, None, Some(item)) => format!("需要物品：{item}"),
        (Language::Chinese, None, None) => generic_locked_message(language),
        (Language::English, Some(title), Some(item)) => {
            format!("Requires scan: {title}, and item: {item}")
        }
        (Language::English, Some(title), None) => format!("Scan required: {title}"),
        (Language::English, None, Some(item)) => format!("Item required: {item}"),
        (Language::English, None, None) => generic_locked_message(language),
    }
}

fn scan_complete_message(language: Language, title: &str) -> String {
    match language {
        Language::Chinese => format!("扫描完成：{title}，研究和奖励已记录"),
        Language::English => format!("Scan complete: {title}. Research logged"),
    }
}

fn notice_colors(tone: NoticeTone, alpha: f32) -> (Color, Color, Color) {
    match tone {
        NoticeTone::Info => (
            Color::rgba(0.015, 0.060, 0.075, 0.82 * alpha),
            Color::rgba(0.42, 0.90, 1.0, 0.88 * alpha),
            Color::rgba(0.86, 1.0, 0.98, alpha),
        ),
        NoticeTone::Success => (
            Color::rgba(0.025, 0.070, 0.045, 0.84 * alpha),
            Color::rgba(0.54, 0.95, 0.58, 0.88 * alpha),
            Color::rgba(0.90, 1.0, 0.90, alpha),
        ),
        NoticeTone::Warning => (
            Color::rgba(0.100, 0.055, 0.020, 0.86 * alpha),
            Color::rgba(1.0, 0.70, 0.30, 0.90 * alpha),
            Color::rgba(1.0, 0.90, 0.72, alpha),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_messages_are_localized() {
        assert_eq!(
            pickup_message(Language::Chinese, "生物样本", 2),
            "获得 生物样本 x2"
        );
        assert_eq!(
            locked_rule_message(Language::English, Some("Locked Door"), None),
            "Scan required: Locked Door"
        );
        assert_eq!(
            locked_rule_message(Language::Chinese, None, Some("遗迹钥匙")),
            "需要物品：遗迹钥匙"
        );
        assert_eq!(stamina_low_message(Language::Chinese), "体力不足，无法跳跃");
        assert_eq!(
            quick_item_used_message(Language::Chinese, "医疗注射器", "生命", 35),
            "使用 医疗注射器，生命 +35"
        );
        assert!(scan_complete_message(Language::Chinese, "终端").contains("扫描完成"));
    }
}
