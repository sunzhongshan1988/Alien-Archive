use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, TryRecvError},
    thread,
};

use anyhow::{Context, Result};
use eframe::egui::{self, Context as EguiContext, TextureHandle, TextureOptions};

use crate::asset_registry::AssetEntry;

const THUMBNAIL_SIZE: u32 = 128;

#[derive(Debug)]
struct ThumbnailRequest {
    id: String,
    path: PathBuf,
}

#[derive(Debug)]
struct DecodedThumbnail {
    size: [usize; 2],
    pixels: Vec<u8>,
}

#[derive(Debug)]
struct ThumbnailResult {
    id: String,
    decoded: Result<DecodedThumbnail, String>,
}

pub(crate) struct ThumbnailLoader {
    request_tx: mpsc::Sender<ThumbnailRequest>,
    result_rx: mpsc::Receiver<ThumbnailResult>,
    requested: HashSet<String>,
    completed: usize,
    failed: usize,
}

impl Default for ThumbnailLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailLoader {
    pub(crate) fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<ThumbnailRequest>();
        let (result_tx, result_rx) = mpsc::channel::<ThumbnailResult>();

        let _ = thread::Builder::new()
            .name("editor-thumbnail-loader".to_owned())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let decoded =
                        decode_thumbnail(&request.path).map_err(|error| format!("{error:#}"));
                    let _ = result_tx.send(ThumbnailResult {
                        id: request.id,
                        decoded,
                    });
                }
            });

        Self {
            request_tx,
            result_rx,
            requested: HashSet::new(),
            completed: 0,
            failed: 0,
        }
    }

    pub(crate) fn request(&mut self, asset: AssetEntry) -> bool {
        if !self.requested.insert(asset.id.clone()) {
            return false;
        }

        let request = ThumbnailRequest {
            id: asset.id,
            path: asset.path,
        };
        if self.request_tx.send(request).is_err() {
            self.completed += 1;
            self.failed += 1;
            return false;
        }
        true
    }

    pub(crate) fn upload_ready(
        &mut self,
        ctx: &EguiContext,
        thumbnails: &mut HashMap<String, TextureHandle>,
        max_uploads: usize,
    ) -> usize {
        let mut uploaded = 0usize;
        for _ in 0..max_uploads {
            let result = match self.result_rx.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };

            self.completed += 1;
            match result.decoded {
                Ok(decoded) => {
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(decoded.size, &decoded.pixels);
                    let texture =
                        ctx.load_texture(&result.id, color_image, TextureOptions::NEAREST);
                    thumbnails.insert(result.id, texture);
                    uploaded += 1;
                }
                Err(error) => {
                    self.failed += 1;
                    eprintln!("failed to load thumbnail {}: {error}", result.id);
                }
            }
        }
        uploaded
    }

    pub(crate) fn has_pending(&self) -> bool {
        self.requested.len() > self.completed
    }

    pub(crate) fn requested_count(&self) -> usize {
        self.requested.len()
    }

    pub(crate) fn completed_count(&self) -> usize {
        self.completed
    }

    pub(crate) fn failed_count(&self) -> usize {
        self.failed
    }
}

fn decode_thumbnail(path: &Path) -> Result<DecodedThumbnail> {
    let image = image::ImageReader::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .decode()
        .with_context(|| format!("failed to decode {}", path.display()))?;
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE).to_rgba8();
    let (width, height) = thumbnail.dimensions();

    Ok(DecodedThumbnail {
        size: [width as usize, height as usize],
        pixels: thumbnail.into_raw(),
    })
}
