use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use runtime::{Color, Rect, Renderer, Vec2};
use rusttype::{Font, Scale, point};

#[derive(Clone)]
pub struct TextSprite {
    pub texture_id: String,
    pub size: Vec2,
}

pub fn upload_text(
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

pub fn load_ui_font() -> Result<Font<'static>> {
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

pub fn draw_text(
    renderer: &mut dyn Renderer,
    text: &TextSprite,
    viewport: Vec2,
    x: f32,
    y: f32,
    color: Color,
) {
    renderer.draw_image(
        &text.texture_id,
        screen_rect(viewport, x, y, text.size.x, text.size.y),
        color,
    );
}

pub fn draw_text_centered(
    renderer: &mut dyn Renderer,
    text: &TextSprite,
    viewport: Vec2,
    center_x: f32,
    y: f32,
    color: Color,
) {
    draw_text(
        renderer,
        text,
        viewport,
        center_x - text.size.x * 0.5,
        y,
        color,
    );
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

fn screen_rect(viewport: Vec2, x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(
        Vec2::new(-viewport.x * 0.5 + x, -viewport.y * 0.5 + y),
        Vec2::new(width, height),
    )
}
