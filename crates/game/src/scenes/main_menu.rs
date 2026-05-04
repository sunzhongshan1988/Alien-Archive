use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use runtime::{Button, Color, InputState, Rect, Renderer, SceneCommand, Vec2};
use rusttype::{Font, Scale, point};

use super::{GameContext, RenderContext, Scene, SceneId};

const BACKGROUND_TEXTURE_ID: &str = "main_menu_background";
const BACKGROUND_PATH: &str = "assets/images/startup/startup_background.png";
const FADE_TIME: f32 = 0.55;
const MENU_PANEL_WIDTH: f32 = 520.0;
const MENU_PANEL_HEIGHT: f32 = 360.0;
const MENU_ITEM_WIDTH: f32 = 320.0;
const MENU_ITEM_HEIGHT: f32 = 54.0;
const MENU_ITEM_GAP: f32 = 18.0;

const MENU_ITEMS: [(&str, MenuAction); 2] = [
    ("开始游戏", MenuAction::StartGame),
    ("退出", MenuAction::Quit),
];

#[derive(Clone, Copy)]
enum MenuAction {
    StartGame,
    Quit,
}

#[derive(Clone)]
struct TextSprite {
    texture_id: String,
    size: Vec2,
}

pub struct MainMenuScene {
    elapsed: f32,
    selected_index: usize,
    title_text: Option<TextSprite>,
    menu_texts: Vec<TextSprite>,
}

impl MainMenuScene {
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            selected_index: 0,
            title_text: None,
            menu_texts: Vec::new(),
        }
    }

    fn confirm_selection(&self) -> SceneCommand<SceneId> {
        match MENU_ITEMS[self.selected_index].1 {
            MenuAction::StartGame => SceneCommand::Switch(SceneId::Game),
            MenuAction::Quit => SceneCommand::Quit,
        }
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
                panel_rect.origin.y + 118.0,
                panel_rect.size.x - 144.0,
                2.0,
            ),
            Color::rgba(0.32, 0.86, 1.0, 0.88),
        );

        if let Some(title) = &self.title_text {
            draw_text_centered(
                ctx.renderer,
                title,
                viewport,
                panel_rect.origin.x + panel_rect.size.x * 0.5,
                panel_rect.origin.y + 42.0,
                Color::rgba(0.78, 0.96, 1.0, 1.0),
            );
        }

        for (index, text) in self.menu_texts.iter().enumerate() {
            let item_rect = self.menu_item_rect(viewport, index);
            let is_selected = index == self.selected_index;

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
        let start_y = panel.origin.y + 160.0 + ((panel.size.y - 200.0) - total_items_height) * 0.5;

        Rect::new(
            Vec2::new(
                panel.origin.x + (panel.size.x - MENU_ITEM_WIDTH) * 0.5,
                start_y + index as f32 * (MENU_ITEM_HEIGHT + MENU_ITEM_GAP),
            ),
            Vec2::new(MENU_ITEM_WIDTH, MENU_ITEM_HEIGHT),
        )
    }

    fn alpha(&self) -> f32 {
        (self.elapsed / FADE_TIME).clamp(0.0, 1.0)
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

        let font = load_ui_font()?;
        self.title_text = Some(upload_text(
            renderer,
            &font,
            "main_menu_title",
            "星尘档案",
            58.0,
        )?);
        self.menu_texts = MENU_ITEMS
            .iter()
            .enumerate()
            .map(|(index, (label, _))| {
                upload_text(
                    renderer,
                    &font,
                    &format!("main_menu_item_{index}"),
                    label,
                    34.0,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn update(
        &mut self,
        _ctx: &mut GameContext,
        dt: f32,
        input: &InputState,
    ) -> Result<SceneCommand<SceneId>> {
        self.elapsed += dt;

        let mut mouse_confirmed_item = false;
        if let Some(cursor_position) = input.cursor_position() {
            let viewport = input.screen_size();
            for index in 0..MENU_ITEMS.len() {
                if screen_point_in_rect(cursor_position, self.menu_item_rect(viewport, index)) {
                    self.selected_index = index;
                    mouse_confirmed_item = input.mouse_left_just_pressed();
                }
            }
        }

        if input.just_pressed(Button::Up) {
            self.selected_index = if self.selected_index == 0 {
                MENU_ITEMS.len() - 1
            } else {
                self.selected_index - 1
            };
        }

        if input.just_pressed(Button::Down) {
            self.selected_index = (self.selected_index + 1) % MENU_ITEMS.len();
        }

        if input.just_pressed(Button::Pause) {
            return Ok(SceneCommand::Quit);
        }

        if input.just_pressed(Button::Confirm)
            || input.just_pressed(Button::Interact)
            || mouse_confirmed_item
        {
            return Ok(self.confirm_selection());
        }

        Ok(SceneCommand::None)
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) -> Result<()> {
        self.draw_main_menu(ctx);
        Ok(())
    }
}

fn upload_text(
    renderer: &mut dyn Renderer,
    font: &Font<'static>,
    texture_id: &str,
    text: &str,
    size: f32,
) -> Result<TextSprite> {
    let (width, height, rgba) = render_text_rgba(font, text, size, 8);
    renderer.load_texture_rgba(texture_id, width, height, &rgba)?;

    Ok(TextSprite {
        texture_id: texture_id.to_owned(),
        size: Vec2::new(width as f32, height as f32),
    })
}

fn render_text_rgba(
    font: &Font<'static>,
    text: &str,
    size: f32,
    padding: i32,
) -> (u32, u32, Vec<u8>) {
    let scale = Scale::uniform(size);
    let metrics = font.v_metrics(scale);
    let glyphs = font
        .layout(text, scale, point(0.0, metrics.ascent))
        .collect::<Vec<_>>();

    let bounds = glyphs
        .iter()
        .filter_map(|glyph| glyph.pixel_bounding_box())
        .fold(None, |bounds, box_| match bounds {
            None => Some((box_.min.x, box_.min.y, box_.max.x, box_.max.y)),
            Some((min_x, min_y, max_x, max_y)) => Some((
                min_x.min(box_.min.x),
                min_y.min(box_.min.y),
                max_x.max(box_.max.x),
                max_y.max(box_.max.y),
            )),
        });

    let Some((min_x, min_y, max_x, max_y)) = bounds else {
        return (1, 1, vec![0, 0, 0, 0]);
    };

    let width = (max_x - min_x + padding * 2).max(1) as u32;
    let height = (max_y - min_y + padding * 2).max(1) as u32;
    let mut rgba = vec![0_u8; width as usize * height as usize * 4];

    for glyph in glyphs {
        let Some(box_) = glyph.pixel_bounding_box() else {
            continue;
        };

        glyph.draw(|x, y, coverage| {
            let x = x + (box_.min.x - min_x + padding) as u32;
            let y = y + (box_.min.y - min_y + padding) as u32;
            let index = (y as usize * width as usize + x as usize) * 4;
            let alpha = (coverage * 255.0).round() as u8;

            rgba[index] = 255;
            rgba[index + 1] = 255;
            rgba[index + 2] = 255;
            rgba[index + 3] = alpha;
        });
    }

    (width, height, rgba)
}

fn load_ui_font() -> Result<Font<'static>> {
    for path in font_candidates() {
        if !path.exists() {
            continue;
        }

        let bytes =
            fs::read(&path).with_context(|| format!("failed to read font {}", path.display()))?;
        if let Some(font) = Font::try_from_vec(bytes) {
            return Ok(font);
        }
    }

    bail!("no usable Chinese UI font found; put one in assets/fonts")
}

fn font_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("assets/fonts/ui.ttf")];
    let mut local_fonts = fs::read_dir("assets/fonts")
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ttf"))
        })
        .collect::<Vec<_>>();

    local_fonts.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if name == "ui.ttf" {
            0
        } else if name.contains("sourcehan") || name.contains("noto") {
            1
        } else {
            2
        }
    });
    candidates.extend(local_fonts);
    candidates.extend([
        PathBuf::from("C:/Windows/Fonts/NotoSansSC-VF.ttf"),
        PathBuf::from("C:/Windows/Fonts/Deng.ttf"),
        PathBuf::from("C:/Windows/Fonts/simhei.ttf"),
    ]);

    candidates
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

fn draw_text_centered(
    renderer: &mut dyn Renderer,
    text: &TextSprite,
    viewport: Vec2,
    center_x: f32,
    y: f32,
    color: Color,
) {
    renderer.draw_image(
        &text.texture_id,
        screen_rect(
            Vec2::ZERO,
            viewport,
            center_x - text.size.x * 0.5,
            y,
            text.size.x,
            text.size.y,
        ),
        color,
    );
}

fn screen_point_in_rect(point: Vec2, rect: Rect) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.right()
        && point.y >= rect.origin.y
        && point.y <= rect.bottom()
}
