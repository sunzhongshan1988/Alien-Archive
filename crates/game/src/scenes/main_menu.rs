use std::path::Path;

use anyhow::Result;
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};

use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, Language, RenderContext, Scene, SceneId};

const BACKGROUND_TEXTURE_ID: &str = "main_menu_background";
const BACKGROUND_PATH: &str = "assets/images/startup/startup_background.png";
const FADE_TIME: f32 = 0.55;
const MENU_PANEL_WIDTH: f32 = 560.0;
const MENU_PANEL_HEIGHT: f32 = 400.0;
const MENU_ITEM_WIDTH: f32 = 320.0;
const MENU_ITEM_HEIGHT: f32 = 54.0;
const MENU_ITEM_GAP: f32 = 18.0;
const SETTINGS_ITEM_HEIGHT: f32 = 60.0;
const SETTINGS_CHOICE_WIDTH: f32 = 128.0;
const SETTINGS_CHOICE_HEIGHT: f32 = 46.0;
const SETTINGS_CHOICE_GAP: f32 = 14.0;

const MENU_ITEMS: [MenuAction; 3] = [
    MenuAction::StartGame,
    MenuAction::Settings,
    MenuAction::Quit,
];
const SETTINGS_ITEMS: [SettingsItem; 2] = [SettingsItem::Language, SettingsItem::Back];

#[derive(Clone, Copy)]
enum MenuAction {
    StartGame,
    Settings,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuPage {
    Main,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsItem {
    Language,
    Back,
}

#[derive(Default)]
struct MainMenuText {
    language: Option<Language>,
    title: Option<TextSprite>,
    main_items: Vec<TextSprite>,
    settings_title: Option<TextSprite>,
    language_label: Option<TextSprite>,
    language_values: Vec<TextSprite>,
    back: Option<TextSprite>,
}

pub struct MainMenuScene {
    elapsed: f32,
    page: MenuPage,
    selected_index: usize,
    language: Language,
    text: MainMenuText,
}

impl MainMenuScene {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            page: MenuPage::Main,
            selected_index: 0,
            language: Language::default(),
            text: MainMenuText::default(),
        }
    }

    fn confirm_main_selection(&mut self) -> SceneCommand<SceneId> {
        match MENU_ITEMS[self.selected_index] {
            MenuAction::StartGame => SceneCommand::Switch(SceneId::Overworld),
            MenuAction::Settings => {
                self.page = MenuPage::Settings;
                self.selected_index = 0;
                SceneCommand::None
            }
            MenuAction::Quit => SceneCommand::Quit,
        }
    }

    fn confirm_settings_selection(&mut self, ctx: &mut GameContext) {
        match SETTINGS_ITEMS[self.selected_index] {
            SettingsItem::Language => self.set_language(ctx, ctx.language.next()),
            SettingsItem::Back => {
                self.page = MenuPage::Main;
                self.selected_index = 1;
            }
        }
    }

    fn set_language(&mut self, ctx: &mut GameContext, language: Language) {
        if ctx.language == language {
            return;
        }

        ctx.language = language;
        self.language = language;
        self.text = MainMenuText::default();
    }

    fn draw_main_menu(&self, ctx: &mut RenderContext<'_>) {
        let viewport = ctx.renderer.screen_size();
        let image_size = ctx
            .renderer
            .texture_size(BACKGROUND_TEXTURE_ID)
            .unwrap_or(viewport);
        let background_rect = cover_rect(Vec2::ZERO, viewport, image_size);

        ctx.renderer.draw_image(
            BACKGROUND_TEXTURE_ID,
            background_rect,
            Color::rgba(1.0, 1.0, 1.0, self.alpha()),
        );
        ctx.renderer.draw_rect(
            screen_rect(Vec2::ZERO, viewport, 0.0, 0.0, viewport.x, viewport.y),
            Color::rgba(0.0, 0.0, 0.0, 0.18),
        );

        let panel_rect = self.menu_panel_rect(viewport);
        ctx.renderer.draw_rect(
            screen_rect(
                Vec2::ZERO,
                viewport,
                panel_rect.origin.x,
                panel_rect.origin.y,
                panel_rect.size.x,
                panel_rect.size.y,
            ),
            Color::rgba(0.015, 0.020, 0.035, 0.62),
        );
        ctx.renderer.draw_rect(
            screen_rect(
                Vec2::ZERO,
                viewport,
                panel_rect.origin.x + 72.0,
                panel_rect.origin.y + 126.0,
                panel_rect.size.x - 144.0,
                2.0,
            ),
            Color::rgba(0.32, 0.86, 1.0, 0.88),
        );

        if let Some(title) = &self.text.title {
            draw_text_centered(
                ctx.renderer,
                title,
                viewport,
                panel_rect.origin.x + panel_rect.size.x * 0.5,
                panel_rect.origin.y + 44.0,
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        match self.page {
            MenuPage::Main => self.draw_main_items(ctx, viewport),
            MenuPage::Settings => self.draw_settings(ctx, viewport),
        }
    }

    fn draw_main_items(&self, ctx: &mut RenderContext<'_>, viewport: Vec2) {
        for (index, text) in self.text.main_items.iter().enumerate() {
            let item_rect = self.menu_item_rect(viewport, index);
            let is_selected = self.page == MenuPage::Main && index == self.selected_index;

            if is_selected {
                ctx.renderer.draw_rect(
                    screen_rect(
                        Vec2::ZERO,
                        viewport,
                        item_rect.origin.x,
                        item_rect.origin.y,
                        item_rect.size.x,
                        item_rect.size.y,
                    ),
                    Color::rgba(0.07, 0.46, 0.70, 0.68),
                );
                ctx.renderer.draw_rect(
                    screen_rect(
                        Vec2::ZERO,
                        viewport,
                        item_rect.origin.x + 16.0,
                        item_rect.origin.y + 15.0,
                        6.0,
                        24.0,
                    ),
                    Color::rgba(0.65, 1.0, 0.88, 1.0),
                );
                ctx.renderer.draw_rect(
                    screen_rect(
                        Vec2::ZERO,
                        viewport,
                        item_rect.origin.x,
                        item_rect.origin.y + item_rect.size.y - 2.0,
                        item_rect.size.x,
                        2.0,
                    ),
                    Color::rgba(0.32, 0.86, 1.0, 0.85),
                );
            }

            let color = if is_selected {
                Color::rgba(0.94, 1.0, 0.98, 1.0)
            } else {
                Color::rgba(0.64, 0.78, 0.84, 0.92)
            };

            draw_text_centered(
                ctx.renderer,
                text,
                viewport,
                item_rect.origin.x + item_rect.size.x * 0.5,
                item_rect.origin.y + 8.0,
                color,
            );
        }
    }

    fn draw_settings(&self, ctx: &mut RenderContext<'_>, viewport: Vec2) {
        let panel_rect = self.menu_panel_rect(viewport);

        if let Some(title) = &self.text.settings_title {
            draw_text_centered(
                ctx.renderer,
                title,
                viewport,
                panel_rect.origin.x + panel_rect.size.x * 0.5,
                panel_rect.origin.y + 146.0,
                Color::rgba(0.78, 0.96, 1.0, 0.98),
            );
        }

        let language_rect = self.settings_item_rect(viewport, 0);
        let language_selected = self.page == MenuPage::Settings
            && SETTINGS_ITEMS[self.selected_index] == SettingsItem::Language;
        if language_selected {
            self.draw_selection_bar(ctx, viewport, language_rect);
        }

        if let Some(label) = &self.text.language_label {
            draw_text(
                ctx.renderer,
                label,
                viewport,
                language_rect.origin.x + 24.0,
                language_rect.origin.y + 11.0,
                if language_selected {
                    Color::rgba(0.92, 1.0, 0.98, 1.0)
                } else {
                    Color::rgba(0.64, 0.78, 0.84, 0.92)
                },
            );
        }

        for (index, language) in Language::SUPPORTED.iter().copied().enumerate() {
            let choice_rect = self.language_choice_rect(viewport, index);
            let active = self.language == language;
            ctx.renderer.draw_rect(
                screen_rect(
                    Vec2::ZERO,
                    viewport,
                    choice_rect.origin.x,
                    choice_rect.origin.y,
                    choice_rect.size.x,
                    choice_rect.size.y,
                ),
                if active {
                    Color::rgba(0.08, 0.44, 0.60, 0.88)
                } else {
                    Color::rgba(0.025, 0.07, 0.10, 0.76)
                },
            );
            if active {
                ctx.renderer.draw_rect(
                    screen_rect(
                        Vec2::ZERO,
                        viewport,
                        choice_rect.origin.x,
                        choice_rect.origin.y + choice_rect.size.y - 2.0,
                        choice_rect.size.x,
                        2.0,
                    ),
                    Color::rgba(0.65, 1.0, 0.88, 1.0),
                );
            }

            if let Some(text) = self.text.language_values.get(index) {
                draw_text_centered(
                    ctx.renderer,
                    text,
                    viewport,
                    choice_rect.origin.x + choice_rect.size.x * 0.5,
                    choice_rect.origin.y + 7.0,
                    if active {
                        Color::rgba(0.94, 1.0, 0.98, 1.0)
                    } else {
                        Color::rgba(0.58, 0.72, 0.78, 0.95)
                    },
                );
            }
        }

        let back_rect = self.settings_item_rect(viewport, 1);
        let back_selected = self.page == MenuPage::Settings
            && SETTINGS_ITEMS[self.selected_index] == SettingsItem::Back;
        if back_selected {
            self.draw_selection_bar(ctx, viewport, back_rect);
        }

        if let Some(back) = &self.text.back {
            draw_text_centered(
                ctx.renderer,
                back,
                viewport,
                back_rect.origin.x + back_rect.size.x * 0.5,
                back_rect.origin.y + 10.0,
                if back_selected {
                    Color::rgba(0.94, 1.0, 0.98, 1.0)
                } else {
                    Color::rgba(0.64, 0.78, 0.84, 0.92)
                },
            );
        }
    }

    fn draw_selection_bar(&self, ctx: &mut RenderContext<'_>, viewport: Vec2, rect: Rect) {
        ctx.renderer.draw_rect(
            screen_rect(
                Vec2::ZERO,
                viewport,
                rect.origin.x,
                rect.origin.y,
                rect.size.x,
                rect.size.y,
            ),
            Color::rgba(0.07, 0.46, 0.70, 0.56),
        );
        ctx.renderer.draw_rect(
            screen_rect(
                Vec2::ZERO,
                viewport,
                rect.origin.x,
                rect.origin.y + rect.size.y - 2.0,
                rect.size.x,
                2.0,
            ),
            Color::rgba(0.32, 0.86, 1.0, 0.85),
        );
    }

    fn menu_panel_rect(&self, viewport: Vec2) -> Rect {
        Rect::new(
            Vec2::new(
                (viewport.x - MENU_PANEL_WIDTH) * 0.5,
                (viewport.y - MENU_PANEL_HEIGHT) * 0.5,
            ),
            Vec2::new(MENU_PANEL_WIDTH, MENU_PANEL_HEIGHT),
        )
    }

    fn menu_item_rect(&self, viewport: Vec2, index: usize) -> Rect {
        let panel = self.menu_panel_rect(viewport);
        let total_items_height = MENU_ITEMS.len() as f32 * MENU_ITEM_HEIGHT
            + (MENU_ITEMS.len() - 1) as f32 * MENU_ITEM_GAP;
        let start_y = panel.origin.y + 152.0 + ((panel.size.y - 206.0) - total_items_height) * 0.5;

        Rect::new(
            Vec2::new(
                panel.origin.x + (panel.size.x - MENU_ITEM_WIDTH) * 0.5,
                start_y + index as f32 * (MENU_ITEM_HEIGHT + MENU_ITEM_GAP),
            ),
            Vec2::new(MENU_ITEM_WIDTH, MENU_ITEM_HEIGHT),
        )
    }

    fn settings_item_rect(&self, viewport: Vec2, index: usize) -> Rect {
        let panel = self.menu_panel_rect(viewport);
        let width = 410.0;
        Rect::new(
            Vec2::new(
                panel.origin.x + (panel.size.x - width) * 0.5,
                panel.origin.y + 190.0 + index as f32 * (SETTINGS_ITEM_HEIGHT + 24.0),
            ),
            Vec2::new(width, SETTINGS_ITEM_HEIGHT),
        )
    }

    fn language_choice_rect(&self, viewport: Vec2, index: usize) -> Rect {
        let row = self.settings_item_rect(viewport, 0);
        let total_width = Language::SUPPORTED.len() as f32 * SETTINGS_CHOICE_WIDTH
            + (Language::SUPPORTED.len() - 1) as f32 * SETTINGS_CHOICE_GAP;
        Rect::new(
            Vec2::new(
                row.origin.x + row.size.x - total_width
                    + index as f32 * (SETTINGS_CHOICE_WIDTH + SETTINGS_CHOICE_GAP),
                row.origin.y + (row.size.y - SETTINGS_CHOICE_HEIGHT) * 0.5,
            ),
            Vec2::new(SETTINGS_CHOICE_WIDTH, SETTINGS_CHOICE_HEIGHT),
        )
    }

    fn alpha(&self) -> f32 {
        (self.elapsed / FADE_TIME).clamp(0.0, 1.0)
    }

    fn current_item_count(&self) -> usize {
        match self.page {
            MenuPage::Main => MENU_ITEMS.len(),
            MenuPage::Settings => SETTINGS_ITEMS.len(),
        }
    }

    fn upload_textures(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        let font = load_ui_font()?;
        let language = self.language;
        self.text = MainMenuText {
            language: Some(language),
            ..MainMenuText::default()
        };

        self.text.title = Some(upload_text(
            renderer,
            &font,
            "main_menu_title",
            main_title(language),
            match language {
                Language::Chinese => 58.0,
                Language::English => 48.0,
            },
        )?);
        self.text.main_items = MENU_ITEMS
            .iter()
            .enumerate()
            .map(|(index, action)| {
                upload_text(
                    renderer,
                    &font,
                    &format!("main_menu_item_{index}"),
                    menu_action_label(*action, language),
                    match language {
                        Language::Chinese => 34.0,
                        Language::English => 30.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.settings_title = Some(upload_text(
            renderer,
            &font,
            "main_menu_settings_title",
            settings_title(language),
            match language {
                Language::Chinese => 30.0,
                Language::English => 28.0,
            },
        )?);
        self.text.language_label = Some(upload_text(
            renderer,
            &font,
            "main_menu_language_label",
            language_setting_label(language),
            match language {
                Language::Chinese => 26.0,
                Language::English => 23.0,
            },
        )?);
        self.text.language_values = Language::SUPPORTED
            .iter()
            .enumerate()
            .map(|(index, language)| {
                upload_text(
                    renderer,
                    &font,
                    &format!("main_menu_language_value_{index}"),
                    language_option_label(*language),
                    24.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        self.text.back = Some(upload_text(
            renderer,
            &font,
            "main_menu_back",
            back_label(language),
            match language {
                Language::Chinese => 30.0,
                Language::English => 28.0,
            },
        )?);

        Ok(())
    }
}

impl Scene for MainMenuScene {
    fn id(&self) -> SceneId {
        SceneId::MainMenu
    }

    fn name(&self) -> &str {
        "MainMenuScene"
    }

    fn setup(&mut self, renderer: &mut dyn Renderer) -> Result<()> {
        renderer.load_texture(BACKGROUND_TEXTURE_ID, Path::new(BACKGROUND_PATH))?;
        self.upload_textures(renderer)
    }

    fn update(
        &mut self,
        ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        self.elapsed += dt;
        if self.language != ctx.language {
            self.language = ctx.language;
            self.text = MainMenuText::default();
        }

        let mut mouse_confirmed_item = false;
        if let Some(cursor_position) = input.cursor_position() {
            let viewport = input.screen_size();
            match self.page {
                MenuPage::Main => {
                    for index in 0..MENU_ITEMS.len() {
                        if screen_point_in_rect(
                            cursor_position,
                            self.menu_item_rect(viewport, index),
                        ) {
                            self.selected_index = index;
                            mouse_confirmed_item = input.mouse_left_just_pressed();
                        }
                    }
                }
                MenuPage::Settings => {
                    for index in 0..SETTINGS_ITEMS.len() {
                        if screen_point_in_rect(
                            cursor_position,
                            self.settings_item_rect(viewport, index),
                        ) {
                            self.selected_index = index;
                            mouse_confirmed_item = input.mouse_left_just_pressed();
                        }
                    }

                    if input.mouse_left_just_pressed() {
                        for (index, language) in Language::SUPPORTED.iter().copied().enumerate() {
                            if screen_point_in_rect(
                                cursor_position,
                                self.language_choice_rect(viewport, index),
                            ) {
                                self.selected_index = 0;
                                self.set_language(ctx, language);
                                mouse_confirmed_item = false;
                            }
                        }
                    }
                }
            }
        }

        if input.just_pressed(Button::Up) {
            let item_count = self.current_item_count();
            self.selected_index = if self.selected_index == 0 {
                item_count - 1
            } else {
                self.selected_index - 1
            };
        }

        if input.just_pressed(Button::Down) {
            self.selected_index = (self.selected_index + 1) % self.current_item_count();
        }

        if self.page == MenuPage::Settings
            && SETTINGS_ITEMS[self.selected_index] == SettingsItem::Language
            && (input.just_pressed(Button::Left) || input.just_pressed(Button::Right))
        {
            self.set_language(ctx, ctx.language.next());
        }

        if input.just_pressed(Button::Pause) {
            if self.page == MenuPage::Settings {
                self.page = MenuPage::Main;
                self.selected_index = 1;
                return Ok(SceneCommand::None);
            }

            return Ok(SceneCommand::Quit);
        }

        if input.just_pressed(Button::Confirm)
            || input.just_pressed(Button::Interact)
            || mouse_confirmed_item
        {
            return Ok(match self.page {
                MenuPage::Main => self.confirm_main_selection(),
                MenuPage::Settings => {
                    self.confirm_settings_selection(ctx);
                    SceneCommand::None
                }
            });
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        if self.text.language != Some(self.language) {
            self.upload_textures(ctx.renderer)?;
        }

        self.draw_main_menu(ctx);
        Ok(())
    }
}

fn cover_rect(center: Vec2, viewport: Vec2, image_size: Vec2) -> Rect {
    let viewport_aspect = viewport.x / viewport.y;
    let image_aspect = image_size.x / image_size.y;
    let size = if image_aspect > viewport_aspect {
        Vec2::new(viewport.y * image_aspect, viewport.y)
    } else {
        Vec2::new(viewport.x, viewport.x / image_aspect)
    };

    Rect::new(
        Vec2::new(center.x - size.x * 0.5, center.y - size.y * 0.5),
        size,
    )
}

fn screen_rect(center: Vec2, viewport: Vec2, x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(
        Vec2::new(
            center.x - viewport.x * 0.5 + x,
            center.y - viewport.y * 0.5 + y,
        ),
        Vec2::new(width, height),
    )
}

fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}

fn main_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "星尘档案",
        Language::English => "Alien Archive",
    }
}

fn menu_action_label(action: MenuAction, language: Language) -> &'static str {
    match language {
        Language::Chinese => match action {
            MenuAction::StartGame => "开始游戏",
            MenuAction::Settings => "设置",
            MenuAction::Quit => "退出",
        },
        Language::English => match action {
            MenuAction::StartGame => "Start Game",
            MenuAction::Settings => "Settings",
            MenuAction::Quit => "Quit",
        },
    }
}

fn settings_title(language: Language) -> &'static str {
    match language {
        Language::Chinese => "设置",
        Language::English => "Settings",
    }
}

fn language_setting_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "语言",
        Language::English => "Language",
    }
}

fn language_option_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "English",
    }
}

fn back_label(language: Language) -> &'static str {
    match language {
        Language::Chinese => "返回",
        Language::English => "Back",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_text_has_chinese_and_english_labels() {
        for language in Language::SUPPORTED {
            assert!(!main_title(language).is_empty());
            assert!(!settings_title(language).is_empty());
            assert!(!language_setting_label(language).is_empty());
            assert!(!language_option_label(language).is_empty());
            assert!(!back_label(language).is_empty());

            for action in MENU_ITEMS {
                assert!(!menu_action_label(action, language).is_empty());
            }
        }
    }
}
