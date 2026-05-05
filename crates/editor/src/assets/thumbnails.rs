use anyhow::{Context, Result};
use eframe::egui::{self, Context as EguiContext, TextureHandle, TextureOptions};

use crate::asset_registry::AssetEntry;

pub(crate) fn load_thumbnail(ctx: &EguiContext, asset: &AssetEntry) -> Result<TextureHandle> {
    let image = image::ImageReader::open(&asset.path)
        .with_context(|| format!("failed to open {}", asset.path.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", asset.path.display()))?;
    let thumbnail = image.thumbnail(128, 128).to_rgba8();
    let (width, height) = thumbnail.dimensions();
    let pixels = thumbnail.into_raw();
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels);

    Ok(ctx.load_texture(&asset.id, color_image, TextureOptions::NEAREST))
}
