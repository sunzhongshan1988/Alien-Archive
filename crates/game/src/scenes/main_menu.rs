use std::path::{Path, PathBuf};

use anyhow::Result;
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};

use crate::save::{SAVE_SLOT_COUNT, SaveData, delete_save_file, save_slot_path};
use crate::ui::menu_style::{self, icon, skin};
use crate::ui::menu_widgets::{
    contain_rect, draw_border, draw_screen_rect, draw_texture_nine_slice, screen_rect as ui_rect,
};
use crate::ui::text::{TextSprite, draw_text, draw_text_centered, load_ui_font, upload_text};

use super::{GameContext, Language, RenderContext, Scene, SceneId};

const BACKGROUND_TEXTURE_ID: &str = "main_menu_background";
const BACKGROUND_PATH: &str = "assets/images/startup/startup_background.png";
const FADE_TIME: f32 = 0.55;
const MENU_PANEL_WIDTH: f32 = 680.0;
const MENU_PANEL_HEIGHT: f32 = 560.0;
const MENU_ITEM_WIDTH: f32 = 430.0;
const MENU_ITEM_HEIGHT: f32 = 52.0;
const MENU_ITEM_GAP: f32 = 10.0;
const SAVE_SLOT_WIDTH: f32 = 500.0;
const SAVE_SLOT_HEIGHT: f32 = 70.0;
const SAVE_SLOT_GAP: f32 = 12.0;
const SETTINGS_ITEM_HEIGHT: f32 = 60.0;
const SETTINGS_CHOICE_WIDTH: f32 = 128.0;
const SETTINGS_CHOICE_HEIGHT: f32 = 46.0;
const SETTINGS_CHOICE_GAP: f32 = 14.0;

const MENU_ITEMS: [MenuAction; 6] = [
    MenuAction::Continue,
    MenuAction::NewGame,
    MenuAction::LoadGame,
    MenuAction::DeleteSave,
    MenuAction::Settings,
    MenuAction::Quit,
];
const SETTINGS_ITEMS: [SettingsItem; 2] = [SettingsItem::Language, SettingsItem::Back];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuAction {
    Continue,
    NewGame,
    LoadGame,
    DeleteSave,
    Settings,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SaveSlotMode {
    Load,
    New,
    Delete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuPage {
    Main,
    Settings,
    SaveSlots(SaveSlotMode),
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
    slot_title: Option<TextSprite>,
    slot_items: Vec<TextSprite>,
}

#[derive(Clone, Debug)]
struct SaveSlotView {
    index: usize,
    path: PathBuf,
    state: SaveSlotState,
}

#[derive(Clone, Debug)]
enum SaveSlotState {
    Empty,
    Ready(SaveData),
    Corrupt,
}

pub struct MainMenuScene {
    elapsed: f32,
    page: MenuPage,
    selected_index: usize,
    language: Language,
    text: MainMenuText,
    save_slots: Vec<SaveSlotView>,
    pending_delete_slot: Option<usize>,
}

impl MainMenuScene {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            page: MenuPage::Main,
            selected_index: 0,
            language: Language::default(),
            text: MainMenuText::default(),
            save_slots: load_save_slots(),
            pending_delete_slot: None,
        }
    }

    fn confirm_main_selection(&mut self, ctx: &mut GameContext) -> Result<SceneCommand<SceneId>> {
        match MENU_ITEMS[self.selected_index] {
            MenuAction::Continue => Ok(SceneCommand::Switch(ctx.resume_scene_id())),
            MenuAction::NewGame => {
                self.open_save_slots(SaveSlotMode::New);
                Ok(SceneCommand::None)
            }
            MenuAction::LoadGame => {
                self.open_save_slots(SaveSlotMode::Load);
                Ok(SceneCommand::None)
            }
            MenuAction::DeleteSave => {
                self.open_save_slots(SaveSlotMode::Delete);
                Ok(SceneCommand::None)
            }
            MenuAction::Settings => {
                self.pending_delete_slot = None;
                self.page = MenuPage::Settings;
                self.selected_index = 0;
                Ok(SceneCommand::None)
            }
            MenuAction::Quit => Ok(SceneCommand::Quit),
        }
    }

    fn open_save_slots(&mut self, mode: SaveSlotMode) {
        self.pending_delete_slot = None;
        self.refresh_save_slots();
        self.page = MenuPage::SaveSlots(mode);
        self.selected_index = 0;
    }

    fn confirm_save_slot_selection(
        &mut self,
        ctx: &mut GameContext,
        mode: SaveSlotMode,
    ) -> Result<SceneCommand<SceneId>> {
        if self.selected_index >= SAVE_SLOT_COUNT {
            self.page = MenuPage::Main;
            self.selected_index = main_index_for_slot_mode(mode);
            self.pending_delete_slot = None;
            return Ok(SceneCommand::None);
        }

        let slot_path = self
            .save_slots
            .get(self.selected_index)
            .map(|slot| slot.path.clone())
            .unwrap_or_else(|| save_slot_path(self.selected_index));
        match mode {
            SaveSlotMode::Load => {
                let has_readable_save = self
                    .save_slots
                    .get(self.selected_index)
                    .is_some_and(|slot| matches!(&slot.state, SaveSlotState::Ready(_)));
                if has_readable_save {
                    ctx.load_save_file_or_default(slot_path);
                } else {
                    ctx.start_new_save(slot_path);
                }
                self.language = ctx.language;
                self.refresh_save_slots();
                Ok(SceneCommand::Switch(ctx.resume_scene_id()))
            }
            SaveSlotMode::New => {
                ctx.start_new_save(slot_path);
                self.language = ctx.language;
                self.refresh_save_slots();
                Ok(SceneCommand::Switch(ctx.resume_scene_id()))
            }
            SaveSlotMode::Delete => {
                let can_delete = self
                    .save_slots
                    .get(self.selected_index)
                    .is_some_and(|slot| !matches!(&slot.state, SaveSlotState::Empty));
                if !can_delete {
                    self.pending_delete_slot = None;
                    self.text = MainMenuText::default();
                    return Ok(SceneCommand::None);
                }

                if self.pending_delete_slot != Some(self.selected_index) {
                    self.pending_delete_slot = Some(self.selected_index);
                    self.text = MainMenuText::default();
                    return Ok(SceneCommand::None);
                }

                self.pending_delete_slot = None;
                delete_save_file(&slot_path)?;
                if ctx.save_path == slot_path {
                    ctx.reset_to_empty_save_slot(slot_path);
                    self.language = ctx.language;
                }
                self.refresh_save_slots();
                Ok(SceneCommand::None)
            }
        }
    }

    fn confirm_settings_selection(&mut self, ctx: &mut GameContext) {
        match SETTINGS_ITEMS[self.selected_index] {
            SettingsItem::Language => self.set_language(ctx, ctx.language.next()),
            SettingsItem::Back => {
                self.page = MenuPage::Main;
                self.selected_index = 4;
                self.pending_delete_slot = None;
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
        ctx.request_save();
    }

    fn refresh_save_slots(&mut self) {
        self.save_slots = load_save_slots();
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
            Color::rgba(0.0, 0.0, 0.0, 0.28),
        );

        let panel_rect = self.menu_panel_rect(viewport);
        self.draw_main_panel(ctx.renderer, viewport, panel_rect);

        if let Some(title) = &self.text.title {
            draw_text_centered(
                ctx.renderer,
                title,
                viewport,
                panel_rect.origin.x + panel_rect.size.x * 0.5,
                centered_text_y(
                    Rect::new(
                        Vec2::new(panel_rect.origin.x, panel_rect.origin.y + 48.0),
                        Vec2::new(panel_rect.size.x, 78.0),
                    ),
                    title,
                    0.0,
                ),
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        match self.page {
            MenuPage::Main => self.draw_main_items(ctx, viewport),
            MenuPage::Settings => self.draw_settings(ctx, viewport),
            MenuPage::SaveSlots(mode) => self.draw_save_slots(ctx, viewport, mode),
        }
    }

    fn draw_main_panel(&self, renderer: &mut dyn Renderer, viewport: Vec2, panel: Rect) {
        let textured = draw_texture_nine_slice(
            renderer,
            viewport,
            skin::PANEL,
            panel,
            46.0,
            28.0,
            Color::rgba(1.0, 1.0, 1.0, 0.96),
        );
        if !textured {
            draw_screen_rect(
                renderer,
                viewport,
                panel,
                Color::rgba(0.015, 0.020, 0.035, 0.78),
            );
            draw_border(
                renderer,
                viewport,
                panel,
                1.0,
                Color::rgba(0.16, 0.52, 0.62, 0.86),
            );
        }

        let logo = Rect::new(
            Vec2::new(panel.origin.x + 44.0, panel.origin.y + 36.0),
            Vec2::new(72.0, 72.0),
        );
        if let Some(image_size) = renderer.texture_size(icon::BRAND_CRYSTAL) {
            renderer.draw_image(
                icon::BRAND_CRYSTAL,
                ui_rect(viewport, contain_rect(logo, image_size)),
                Color::rgba(1.0, 1.0, 1.0, 0.96),
            );
        }

        draw_screen_rect(
            renderer,
            viewport,
            Rect::new(
                Vec2::new(panel.origin.x + 72.0, panel.origin.y + 134.0),
                Vec2::new(panel.size.x - 144.0, 2.0),
            ),
            Color::rgba(0.32, 0.86, 1.0, 0.68),
        );
    }

    fn draw_main_items(&self, ctx: &mut RenderContext<'_>, viewport: Vec2) {
        for (index, text) in self.text.main_items.iter().enumerate() {
            let item_rect = self.menu_item_rect(viewport, index);
            let is_selected = self.page == MenuPage::Main && index == self.selected_index;
            draw_menu_button(ctx.renderer, viewport, item_rect, is_selected);

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
                centered_text_y(item_rect, text, 0.0),
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
                centered_text_y(
                    Rect::new(
                        Vec2::new(panel_rect.origin.x, panel_rect.origin.y + 136.0),
                        Vec2::new(panel_rect.size.x, 48.0),
                    ),
                    title,
                    0.0,
                ),
                Color::rgba(0.78, 0.96, 1.0, 0.98),
            );
        }

        let language_rect = self.settings_item_rect(viewport, 0);
        let language_selected = self.page == MenuPage::Settings
            && SETTINGS_ITEMS[self.selected_index] == SettingsItem::Language;
        if language_selected {
            draw_menu_button(ctx.renderer, viewport, language_rect, true);
        } else {
            draw_menu_button(ctx.renderer, viewport, language_rect, false);
        }

        if let Some(label) = &self.text.language_label {
            draw_text(
                ctx.renderer,
                label,
                viewport,
                language_rect.origin.x + 24.0,
                centered_text_y(language_rect, label, 0.0),
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
            draw_language_button(ctx.renderer, viewport, choice_rect, active);

            if let Some(text) = self.text.language_values.get(index) {
                draw_text_centered(
                    ctx.renderer,
                    text,
                    viewport,
                    choice_rect.origin.x + choice_rect.size.x * 0.5,
                    centered_text_y(choice_rect, text, 0.0),
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
        draw_menu_button(ctx.renderer, viewport, back_rect, back_selected);

        if let Some(back) = &self.text.back {
            draw_text_centered(
                ctx.renderer,
                back,
                viewport,
                back_rect.origin.x + back_rect.size.x * 0.5,
                centered_text_y(back_rect, back, 0.0),
                if back_selected {
                    Color::rgba(0.94, 1.0, 0.98, 1.0)
                } else {
                    Color::rgba(0.64, 0.78, 0.84, 0.92)
                },
            );
        }
    }

    fn draw_save_slots(&self, ctx: &mut RenderContext<'_>, viewport: Vec2, mode: SaveSlotMode) {
        let panel_rect = self.menu_panel_rect(viewport);

        if let Some(title) = &self.text.slot_title {
            draw_text_centered(
                ctx.renderer,
                title,
                viewport,
                panel_rect.origin.x + panel_rect.size.x * 0.5,
                centered_text_y(
                    Rect::new(
                        Vec2::new(panel_rect.origin.x, panel_rect.origin.y + 136.0),
                        Vec2::new(panel_rect.size.x, 48.0),
                    ),
                    title,
                    0.0,
                ),
                Color::rgba(0.78, 0.96, 1.0, 0.98),
            );
        }

        for index in 0..=SAVE_SLOT_COUNT {
            let item_rect = self.save_slot_item_rect(viewport, index);
            let is_selected =
                self.page == MenuPage::SaveSlots(mode) && index == self.selected_index;
            draw_menu_button(ctx.renderer, viewport, item_rect, is_selected);

            if let Some(text) = self.text.slot_items.get(index) {
                let color = if is_selected {
                    Color::rgba(0.94, 1.0, 0.98, 1.0)
                } else if matches!(mode, SaveSlotMode::Delete) && index < SAVE_SLOT_COUNT {
                    Color::rgba(1.0, 0.78, 0.66, 0.95)
                } else {
                    Color::rgba(0.64, 0.78, 0.84, 0.92)
                };
                draw_text_centered(
                    ctx.renderer,
                    text,
                    viewport,
                    item_rect.origin.x + item_rect.size.x * 0.5,
                    centered_text_y(item_rect, text, 0.0),
                    color,
                );
            }
        }
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

    fn save_slot_item_rect(&self, viewport: Vec2, index: usize) -> Rect {
        let panel = self.menu_panel_rect(viewport);
        Rect::new(
            Vec2::new(
                panel.origin.x + (panel.size.x - SAVE_SLOT_WIDTH) * 0.5,
                panel.origin.y + 188.0 + index as f32 * (SAVE_SLOT_HEIGHT + SAVE_SLOT_GAP),
            ),
            Vec2::new(SAVE_SLOT_WIDTH, SAVE_SLOT_HEIGHT),
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
            MenuPage::SaveSlots(_) => SAVE_SLOT_COUNT + 1,
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
        self.text.slot_title = Some(upload_text(
            renderer,
            &font,
            "main_menu_slot_title",
            save_slot_title(save_slot_mode_for_page(self.page), language),
            match language {
                Language::Chinese => 30.0,
                Language::English => 26.0,
            },
        )?);
        let mode = save_slot_mode_for_page(self.page);
        self.text.slot_items = (0..=SAVE_SLOT_COUNT)
            .map(|index| {
                let label = if index < SAVE_SLOT_COUNT {
                    save_slot_item_label(
                        self.save_slots.get(index),
                        mode,
                        language,
                        self.pending_delete_slot == Some(index),
                    )
                } else {
                    back_label(language).to_owned()
                };
                upload_text(
                    renderer,
                    &font,
                    &format!("main_menu_slot_item_{index}"),
                    &label,
                    match language {
                        Language::Chinese => 23.0,
                        Language::English => 20.0,
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;

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
        load_menu_textures(renderer)?;
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
                MenuPage::SaveSlots(_) => {
                    for index in 0..=SAVE_SLOT_COUNT {
                        if screen_point_in_rect(
                            cursor_position,
                            self.save_slot_item_rect(viewport, index),
                        ) {
                            self.selected_index = index;
                            mouse_confirmed_item = input.mouse_left_just_pressed();
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
            match self.page {
                MenuPage::Main => return Ok(SceneCommand::Quit),
                MenuPage::Settings => {
                    self.page = MenuPage::Main;
                    self.selected_index = 4;
                    return Ok(SceneCommand::None);
                }
                MenuPage::SaveSlots(mode) => {
                    self.page = MenuPage::Main;
                    self.selected_index = main_index_for_slot_mode(mode);
                    self.pending_delete_slot = None;
                    return Ok(SceneCommand::None);
                }
            }
        }

        if input.just_pressed(Button::Confirm)
            || input.just_pressed(Button::Interact)
            || mouse_confirmed_item
        {
            return match self.page {
                MenuPage::Main => self.confirm_main_selection(ctx),
                MenuPage::Settings => {
                    self.confirm_settings_selection(ctx);
                    Ok(SceneCommand::None)
                }
                MenuPage::SaveSlots(mode) => self.confirm_save_slot_selection(ctx, mode),
            };
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

fn load_menu_textures(renderer: &mut dyn Renderer) -> Result<()> {
    for texture in menu_style::TEXTURES {
        if renderer.texture_size(texture.texture_id).is_none() {
            renderer.load_texture(texture.texture_id, Path::new(texture.path))?;
        }
    }

    Ok(())
}

fn draw_menu_button(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, selected: bool) {
    let texture_id = if selected {
        skin::NAV_ACTIVE
    } else {
        skin::NAV_IDLE
    };
    let textured = draw_texture_nine_slice(
        renderer,
        viewport,
        texture_id,
        rect,
        42.0,
        18.0,
        Color::rgba(1.0, 1.0, 1.0, if selected { 0.96 } else { 0.82 }),
    );
    if !textured {
        draw_screen_rect(
            renderer,
            viewport,
            rect,
            if selected {
                Color::rgba(0.07, 0.46, 0.70, 0.68)
            } else {
                Color::rgba(0.015, 0.045, 0.058, 0.68)
            },
        );
        draw_border(
            renderer,
            viewport,
            rect,
            1.0,
            if selected {
                Color::rgba(0.52, 0.96, 1.0, 0.92)
            } else {
                Color::rgba(0.12, 0.27, 0.35, 0.86)
            },
        );
    }
}

fn draw_language_button(renderer: &mut dyn Renderer, viewport: Vec2, rect: Rect, active: bool) {
    let texture_id = if active {
        skin::NAV_ACTIVE
    } else {
        skin::LANGUAGE_TOGGLE
    };
    let textured = draw_texture_nine_slice(
        renderer,
        viewport,
        texture_id,
        rect,
        46.0,
        16.0,
        Color::rgba(1.0, 1.0, 1.0, if active { 0.96 } else { 0.82 }),
    );
    if !textured {
        draw_screen_rect(
            renderer,
            viewport,
            rect,
            if active {
                Color::rgba(0.08, 0.44, 0.60, 0.88)
            } else {
                Color::rgba(0.025, 0.07, 0.10, 0.76)
            },
        );
    }
}

fn centered_text_y(rect: Rect, text: &TextSprite, y_offset: f32) -> f32 {
    let text_padding = 8.0;
    let visual_height = (text.size.y - text_padding * 2.0).max(0.0);
    rect.origin.y + (rect.size.y - visual_height) * 0.5 + y_offset - text_padding
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
            MenuAction::Continue => "继续游戏",
            MenuAction::NewGame => "新建存档",
            MenuAction::LoadGame => "读取存档",
            MenuAction::DeleteSave => "删除存档",
            MenuAction::Settings => "设置",
            MenuAction::Quit => "退出",
        },
        Language::English => match action {
            MenuAction::Continue => "Continue",
            MenuAction::NewGame => "New Save",
            MenuAction::LoadGame => "Load Save",
            MenuAction::DeleteSave => "Delete Save",
            MenuAction::Settings => "Settings",
            MenuAction::Quit => "Quit",
        },
    }
}

fn main_index_for_slot_mode(mode: SaveSlotMode) -> usize {
    match mode {
        SaveSlotMode::Load => 2,
        SaveSlotMode::New => 1,
        SaveSlotMode::Delete => 3,
    }
}

fn load_save_slots() -> Vec<SaveSlotView> {
    (0..SAVE_SLOT_COUNT)
        .map(|index| {
            let path = save_slot_path(index);
            let state = if !path.exists() {
                SaveSlotState::Empty
            } else {
                match SaveData::load(&path) {
                    Ok(save_data) => SaveSlotState::Ready(save_data),
                    Err(_) => SaveSlotState::Corrupt,
                }
            };
            SaveSlotView { index, path, state }
        })
        .collect()
}

fn save_slot_mode_for_page(page: MenuPage) -> SaveSlotMode {
    match page {
        MenuPage::SaveSlots(mode) => mode,
        _ => SaveSlotMode::Load,
    }
}

fn save_slot_title(mode: SaveSlotMode, language: Language) -> &'static str {
    match language {
        Language::Chinese => match mode {
            SaveSlotMode::Load => "读取存档",
            SaveSlotMode::New => "新建存档",
            SaveSlotMode::Delete => "删除存档",
        },
        Language::English => match mode {
            SaveSlotMode::Load => "Load Save",
            SaveSlotMode::New => "New Save",
            SaveSlotMode::Delete => "Delete Save",
        },
    }
}

fn save_slot_item_label(
    slot: Option<&SaveSlotView>,
    mode: SaveSlotMode,
    language: Language,
    confirm_delete: bool,
) -> String {
    let Some(slot) = slot else {
        return match language {
            Language::Chinese => "未知槽位".to_owned(),
            Language::English => "Unknown Slot".to_owned(),
        };
    };
    let slot_name = match language {
        Language::Chinese => format!("槽位 {}", slot.index + 1),
        Language::English => format!("Slot {}", slot.index + 1),
    };

    if mode == SaveSlotMode::Delete
        && confirm_delete
        && !matches!(&slot.state, SaveSlotState::Empty)
    {
        return confirm_delete_slot_label(&slot_name, language);
    }

    match &slot.state {
        SaveSlotState::Empty => empty_slot_label(&slot_name, mode, language),
        SaveSlotState::Corrupt => corrupt_slot_label(&slot_name, mode, language),
        SaveSlotState::Ready(save_data) => ready_slot_label(&slot_name, save_data, mode, language),
    }
}

fn confirm_delete_slot_label(slot_name: &str, language: Language) -> String {
    match language {
        Language::Chinese => format!("{slot_name}  再次确认删除"),
        Language::English => format!("{slot_name}  Confirm Delete"),
    }
}

fn empty_slot_label(slot_name: &str, mode: SaveSlotMode, language: Language) -> String {
    match language {
        Language::Chinese => match mode {
            SaveSlotMode::Load | SaveSlotMode::New => format!("{slot_name}  空 - 新游戏"),
            SaveSlotMode::Delete => format!("{slot_name}  空"),
        },
        Language::English => match mode {
            SaveSlotMode::Load | SaveSlotMode::New => format!("{slot_name}  Empty - New Game"),
            SaveSlotMode::Delete => format!("{slot_name}  Empty"),
        },
    }
}

fn corrupt_slot_label(slot_name: &str, mode: SaveSlotMode, language: Language) -> String {
    match language {
        Language::Chinese => match mode {
            SaveSlotMode::Delete => format!("{slot_name}  读取失败 - 删除"),
            SaveSlotMode::Load | SaveSlotMode::New => format!("{slot_name}  读取失败 - 重建"),
        },
        Language::English => match mode {
            SaveSlotMode::Delete => format!("{slot_name}  Read Failed - Delete"),
            SaveSlotMode::Load | SaveSlotMode::New => format!("{slot_name}  Read Failed - Rebuild"),
        },
    }
}

fn ready_slot_label(
    slot_name: &str,
    save_data: &SaveData,
    mode: SaveSlotMode,
    language: Language,
) -> String {
    let scene = scene_display_name(&save_data.world.current_scene, language);
    let scanned_count = save_data.codex.scanned_ids.len();
    match language {
        Language::Chinese => match mode {
            SaveSlotMode::Load => format!(
                "{slot_name}  Lv{}  {scene}  扫描 {scanned_count}",
                save_data.profile.level
            ),
            SaveSlotMode::New => {
                format!("{slot_name}  Lv{}  {scene}  覆盖", save_data.profile.level)
            }
            SaveSlotMode::Delete => {
                format!("{slot_name}  Lv{}  {scene}  删除", save_data.profile.level)
            }
        },
        Language::English => match mode {
            SaveSlotMode::Load => format!(
                "{slot_name}  Lv{}  {scene}  Scans {scanned_count}",
                save_data.profile.level
            ),
            SaveSlotMode::New => format!(
                "{slot_name}  Lv{}  {scene}  Overwrite",
                save_data.profile.level
            ),
            SaveSlotMode::Delete => format!(
                "{slot_name}  Lv{}  {scene}  Delete",
                save_data.profile.level
            ),
        },
    }
}

fn scene_display_name(scene: &str, language: Language) -> &'static str {
    match language {
        Language::Chinese => match scene {
            "Facility" | "facility" => "设施",
            _ => "地表",
        },
        Language::English => match scene {
            "Facility" | "facility" => "Facility",
            _ => "Overworld",
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
            assert!(!save_slot_title(SaveSlotMode::Load, language).is_empty());
            assert!(!save_slot_title(SaveSlotMode::New, language).is_empty());
            assert!(!save_slot_title(SaveSlotMode::Delete, language).is_empty());

            for action in MENU_ITEMS {
                assert!(!menu_action_label(action, language).is_empty());
            }
        }
    }

    #[test]
    fn save_slot_labels_describe_empty_and_ready_slots() {
        let empty_slot = SaveSlotView {
            index: 0,
            path: PathBuf::from("saves/profile_01.ron"),
            state: SaveSlotState::Empty,
        };
        assert!(
            save_slot_item_label(
                Some(&empty_slot),
                SaveSlotMode::New,
                Language::Chinese,
                false,
            )
            .contains("新游戏")
        );

        let mut save_data = SaveData::default();
        save_data.profile.level = 7;
        save_data.world.current_scene = "Facility".to_owned();
        save_data
            .codex
            .scanned_ids
            .insert("codex.flora.glowfungus".to_owned());
        let ready_slot = SaveSlotView {
            index: 1,
            path: PathBuf::from("saves/profile_02.ron"),
            state: SaveSlotState::Ready(save_data),
        };
        let label = save_slot_item_label(
            Some(&ready_slot),
            SaveSlotMode::Load,
            Language::Chinese,
            false,
        );
        assert!(label.contains("Lv7"));
        assert!(label.contains("设施"));
        assert!(label.contains("扫描 1"));
        assert!(
            save_slot_item_label(
                Some(&ready_slot),
                SaveSlotMode::Delete,
                Language::Chinese,
                true,
            )
            .contains("再次确认删除")
        );
    }
}
