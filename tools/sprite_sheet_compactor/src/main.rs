use std::collections::VecDeque;
use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use image::{
    Rgba, RgbaImage,
    imageops::{self, FilterType},
};

fn main() -> Result<()> {
    let config = Config::from_args(env::args().skip(1))?;
    let source = image::open(&config.input)
        .with_context(|| format!("failed to open {}", config.input.display()))?
        .to_rgba8();

    let has_alpha = source
        .pixels()
        .any(|pixel| pixel[3] <= config.alpha_threshold);
    let input_columns = config.input_columns.unwrap_or(config.frames);
    if input_columns == 0 {
        bail!("input columns must be greater than zero");
    }

    let input_rows = config
        .input_rows
        .unwrap_or_else(|| div_ceil(config.frames, input_columns));

    if input_rows == 0 {
        bail!("input rows must be greater than zero");
    }
    if input_columns * input_rows < config.frames {
        bail!(
            "input grid {}x{} cannot hold {} frames",
            input_columns,
            input_rows,
            config.frames
        );
    }
    if source.width() % input_columns != 0 || source.height() % input_rows != 0 {
        bail!(
            "image size {}x{} is not evenly divisible by input grid {}x{}",
            source.width(),
            source.height(),
            input_columns,
            input_rows
        );
    }

    let slot_width = source.width() / input_columns;
    let slot_height = source.height() / input_rows;
    let output_columns = config.output_columns.unwrap_or(input_columns);
    if output_columns == 0 {
        bail!("output columns must be greater than zero");
    }

    if config.preserve_canvas {
        if config.output_cell_size.is_some() {
            bail!("--preserve-canvas cannot be combined with --cell-size or --frame-size");
        }

        let output = clean_sheet_preserving_canvas(
            &source,
            input_columns,
            output_columns,
            slot_width,
            slot_height,
            has_alpha,
            &config,
        );
        save_output(&output, &config.output)?;

        println!(
            "input: {}x{}; slot: {}x{}; frames: {}",
            source.width(),
            source.height(),
            slot_width,
            slot_height,
            config.frames
        );
        println!(
            "preserved canvas; output cell: {}x{}",
            slot_width, slot_height
        );
        println!(
            "output: {}x{}; saved: {}",
            output.width(),
            output.height(),
            config.output.display()
        );

        return Ok(());
    }

    let mut frames = Vec::new();

    for frame_index in 0..config.frames {
        let column = frame_index % input_columns;
        let row = frame_index / input_columns;
        frames.push(extract_frame(
            &source,
            column * slot_width,
            row * slot_height,
            slot_width,
            slot_height,
            has_alpha,
            &config,
        ));
    }

    let max_frame_width = frames
        .iter()
        .filter_map(|frame| frame.bounds.map(|_| frame.image.width()))
        .max()
        .unwrap_or(1);
    let max_frame_height = frames
        .iter()
        .filter_map(|frame| frame.bounds.map(|_| frame.image.height()))
        .max()
        .unwrap_or(1);

    let output_rows = div_ceil(config.frames, output_columns);
    let auto_cell_width = max_frame_width + config.padding * 2;
    let auto_cell_height = max_frame_height + config.padding * 2;
    let cell_size = config.output_cell_size.unwrap_or(Size {
        width: auto_cell_width,
        height: auto_cell_height,
    });

    if cell_size.width <= config.padding * 2 || cell_size.height <= config.padding * 2 {
        bail!(
            "cell size {}x{} is too small for {} px padding",
            cell_size.width,
            cell_size.height,
            config.padding
        );
    }

    let scale = if config.output_cell_size.is_some() {
        let available_width = cell_size.width - config.padding * 2;
        let available_height = cell_size.height - config.padding * 2;
        let scale_x = available_width as f32 / max_frame_width as f32;
        let scale_y = available_height as f32 / max_frame_height as f32;
        scale_x.min(scale_y)
    } else {
        1.0
    };

    let mut output = RgbaImage::from_pixel(
        cell_size.width * output_columns,
        cell_size.height * output_rows,
        Rgba([0, 0, 0, 0]),
    );

    for (index, frame) in frames.iter().enumerate() {
        if frame.bounds.is_none() {
            continue;
        }

        let frame_image = prepare_frame_image(frame, scale, &cell_size, &config);
        let column = index as u32 % output_columns;
        let row = index as u32 / output_columns;
        let x = column * cell_size.width
            + config.padding
            + (cell_size.width - config.padding * 2 - frame_image.width()) / 2;
        let y = row * cell_size.height
            + config.padding
            + (cell_size.height - config.padding * 2 - frame_image.height());
        imageops::overlay(&mut output, &frame_image, x.into(), y.into());
    }

    save_output(&output, &config.output)?;

    println!(
        "input: {}x{}; slot: {}x{}; frames: {}",
        source.width(),
        source.height(),
        slot_width,
        slot_height,
        config.frames
    );
    println!(
        "trimmed max frame: {}x{}; output cell: {}x{}",
        max_frame_width, max_frame_height, cell_size.width, cell_size.height
    );
    if config.stretch_to_cell {
        println!("stretched to cell; filter: {}", config.filter.name());
    } else if config.output_cell_size.is_some() {
        println!(
            "shared scale: {:.4}; filter: {}",
            scale,
            config.filter.name()
        );
    }
    println!(
        "output: {}x{}; saved: {}",
        output.width(),
        output.height(),
        config.output.display()
    );

    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BackgroundMode {
    Auto,
    Transparent,
    White,
}

struct Config {
    input: PathBuf,
    output: PathBuf,
    frames: u32,
    input_columns: Option<u32>,
    input_rows: Option<u32>,
    output_columns: Option<u32>,
    output_cell_size: Option<Size>,
    padding: u32,
    filter: ResizeFilter,
    preserve_canvas: bool,
    fill_transparent: bool,
    stretch_to_cell: bool,
    background_mode: BackgroundMode,
    alpha_threshold: u8,
    white_threshold: u8,
    white_chroma: u8,
    keep_background: bool,
}

impl Config {
    fn from_args(args: impl Iterator<Item = String>) -> Result<Self> {
        let mut config = Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            frames: 0,
            input_columns: None,
            input_rows: None,
            output_columns: None,
            output_cell_size: None,
            padding: 4,
            filter: ResizeFilter::Nearest,
            preserve_canvas: false,
            fill_transparent: false,
            stretch_to_cell: false,
            background_mode: BackgroundMode::Auto,
            alpha_threshold: 8,
            white_threshold: 225,
            white_chroma: 28,
            keep_background: false,
        };

        let mut args = args.peekable();
        if args.peek().is_none() {
            print_help();
            bail!("missing arguments");
        }

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                "-i" | "--input" => config.input = next_path(&mut args, &arg)?,
                "-o" | "--output" => config.output = next_path(&mut args, &arg)?,
                "-f" | "--frames" => config.frames = next_u32(&mut args, &arg)?,
                "--input-columns" => config.input_columns = Some(next_u32(&mut args, &arg)?),
                "--input-rows" => config.input_rows = Some(next_u32(&mut args, &arg)?),
                "--output-columns" => config.output_columns = Some(next_u32(&mut args, &arg)?),
                "--cell-size" | "--frame-size" => {
                    config.output_cell_size = Some(parse_size(&next_string(&mut args, &arg)?)?)
                }
                "-p" | "--padding" => config.padding = next_u32(&mut args, &arg)?,
                "--filter" => {
                    config.filter = match next_string(&mut args, &arg)?.as_str() {
                        "nearest" => ResizeFilter::Nearest,
                        "triangle" => ResizeFilter::Triangle,
                        "catmullrom" => ResizeFilter::CatmullRom,
                        "gaussian" => ResizeFilter::Gaussian,
                        "lanczos3" => ResizeFilter::Lanczos3,
                        value => bail!("unknown resize filter: {value}"),
                    }
                }
                "--background" => {
                    config.background_mode = match next_string(&mut args, &arg)?.as_str() {
                        "auto" => BackgroundMode::Auto,
                        "transparent" => BackgroundMode::Transparent,
                        "white" => BackgroundMode::White,
                        value => bail!("unknown background mode: {value}"),
                    }
                }
                "--alpha-threshold" => config.alpha_threshold = next_u8(&mut args, &arg)?,
                "--white-threshold" => config.white_threshold = next_u8(&mut args, &arg)?,
                "--white-chroma" => config.white_chroma = next_u8(&mut args, &arg)?,
                "--preserve-canvas" => config.preserve_canvas = true,
                "--fill-transparent" => config.fill_transparent = true,
                "--stretch-to-cell" => config.stretch_to_cell = true,
                "--keep-background" => config.keep_background = true,
                value => bail!("unknown argument: {value}"),
            }
        }

        if config.input.as_os_str().is_empty() {
            bail!("--input is required");
        }
        if config.output.as_os_str().is_empty() {
            bail!("--output is required");
        }
        if config.frames == 0 {
            bail!("--frames must be greater than zero");
        }
        if config.stretch_to_cell && config.output_cell_size.is_none() {
            bail!("--stretch-to-cell requires --cell-size or --frame-size");
        }

        Ok(config)
    }
}

#[derive(Clone, Copy)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl ResizeFilter {
    fn filter_type(self) -> FilterType {
        match self {
            Self::Nearest => FilterType::Nearest,
            Self::Triangle => FilterType::Triangle,
            Self::CatmullRom => FilterType::CatmullRom,
            Self::Gaussian => FilterType::Gaussian,
            Self::Lanczos3 => FilterType::Lanczos3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Nearest => "nearest",
            Self::Triangle => "triangle",
            Self::CatmullRom => "catmullrom",
            Self::Gaussian => "gaussian",
            Self::Lanczos3 => "lanczos3",
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn prepare_frame_image(
    frame: &ExtractedFrame,
    scale: f32,
    cell_size: &Size,
    config: &Config,
) -> RgbaImage {
    let mut image = if config.fill_transparent {
        fill_transparent_with_nearest(&frame.image, config.alpha_threshold)
    } else {
        frame.image.clone()
    };

    if config.stretch_to_cell {
        let width = cell_size.width - config.padding * 2;
        let height = cell_size.height - config.padding * 2;
        return imageops::resize(&image, width, height, config.filter.filter_type());
    }

    if (scale - 1.0).abs() > f32::EPSILON {
        let width = ((image.width() as f32 * scale).round() as u32).max(1);
        let height = ((image.height() as f32 * scale).round() as u32).max(1);
        image = imageops::resize(&image, width, height, config.filter.filter_type());
    }

    image
}

fn fill_transparent_with_nearest(image: &RgbaImage, alpha_threshold: u8) -> RgbaImage {
    let width = image.width();
    let height = image.height();
    let mut output = image.clone();
    let mut filled = vec![false; (width * height) as usize];
    let mut queue = VecDeque::new();

    for y in 0..height {
        for x in 0..width {
            let index = index_of(x, y, width);
            if image.get_pixel(x, y)[3] > alpha_threshold {
                filled[index] = true;
                let pixel = output.get_pixel_mut(x, y);
                pixel[3] = 255;
                queue.push_back((x, y));
            }
        }
    }

    if queue.is_empty() {
        return output;
    }

    while let Some((x, y)) = queue.pop_front() {
        let color = *output.get_pixel(x, y);
        for (next_x, next_y) in neighbors4(x, y, width, height) {
            let index = index_of(next_x, next_y, width);
            if filled[index] {
                continue;
            }

            output.put_pixel(next_x, next_y, color);
            filled[index] = true;
            queue.push_back((next_x, next_y));
        }
    }

    output
}

fn neighbors4(x: u32, y: u32, width: u32, height: u32) -> impl Iterator<Item = (u32, u32)> {
    let mut neighbors = [(0, 0); 4];
    let mut count = 0;

    if x > 0 {
        neighbors[count] = (x - 1, y);
        count += 1;
    }
    if x + 1 < width {
        neighbors[count] = (x + 1, y);
        count += 1;
    }
    if y > 0 {
        neighbors[count] = (x, y - 1);
        count += 1;
    }
    if y + 1 < height {
        neighbors[count] = (x, y + 1);
        count += 1;
    }

    neighbors.into_iter().take(count)
}

fn save_output(output: &RgbaImage, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    output
        .save(path)
        .with_context(|| format!("failed to save {}", path.display()))
}

struct ExtractedFrame {
    image: RgbaImage,
    bounds: Option<Rect>,
}

fn clean_sheet_preserving_canvas(
    source: &RgbaImage,
    input_columns: u32,
    output_columns: u32,
    slot_width: u32,
    slot_height: u32,
    has_alpha: bool,
    config: &Config,
) -> RgbaImage {
    let output_rows = div_ceil(config.frames, output_columns);
    let mut output = RgbaImage::from_pixel(
        slot_width * output_columns,
        slot_height * output_rows,
        Rgba([0, 0, 0, 0]),
    );

    for frame_index in 0..config.frames {
        let source_column = frame_index % input_columns;
        let source_row = frame_index / input_columns;
        let output_column = frame_index % output_columns;
        let output_row = frame_index / output_columns;
        let cleaned = clean_slot_preserving_canvas(
            source,
            source_column * slot_width,
            source_row * slot_height,
            slot_width,
            slot_height,
            has_alpha,
            config,
        );

        imageops::overlay(
            &mut output,
            &cleaned,
            (output_column * slot_width).into(),
            (output_row * slot_height).into(),
        );
    }

    output
}

fn clean_slot_preserving_canvas(
    source: &RgbaImage,
    slot_x: u32,
    slot_y: u32,
    slot_width: u32,
    slot_height: u32,
    has_alpha: bool,
    config: &Config,
) -> RgbaImage {
    let background = build_background_mask(
        source,
        slot_x,
        slot_y,
        slot_width,
        slot_height,
        has_alpha,
        config,
    );
    let mut image = RgbaImage::new(slot_width, slot_height);

    for y in 0..slot_height {
        for x in 0..slot_width {
            let source_pixel = *source.get_pixel(slot_x + x, slot_y + y);
            if background[index_of(x, y, slot_width)] && !config.keep_background {
                image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                image.put_pixel(x, y, source_pixel);
            }
        }
    }

    image
}

fn extract_frame(
    source: &RgbaImage,
    slot_x: u32,
    slot_y: u32,
    slot_width: u32,
    slot_height: u32,
    has_alpha: bool,
    config: &Config,
) -> ExtractedFrame {
    let background = build_background_mask(
        source,
        slot_x,
        slot_y,
        slot_width,
        slot_height,
        has_alpha,
        config,
    );

    let mut min_x = slot_width;
    let mut min_y = slot_height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found_content = false;

    for y in 0..slot_height {
        for x in 0..slot_width {
            if !background[index_of(x, y, slot_width)] {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found_content = true;
            }
        }
    }

    if !found_content {
        return ExtractedFrame {
            image: RgbaImage::new(1, 1),
            bounds: None,
        };
    }

    let bounds = Rect {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    };
    let mut image = RgbaImage::new(bounds.width, bounds.height);

    for y in 0..bounds.height {
        for x in 0..bounds.width {
            let source_x = slot_x + bounds.x + x;
            let source_y = slot_y + bounds.y + y;
            let source_pixel = *source.get_pixel(source_x, source_y);
            let is_background = background[index_of(bounds.x + x, bounds.y + y, slot_width)];

            if is_background && !config.keep_background {
                image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            } else {
                image.put_pixel(x, y, source_pixel);
            }
        }
    }

    ExtractedFrame {
        image,
        bounds: Some(bounds),
    }
}

fn build_background_mask(
    source: &RgbaImage,
    slot_x: u32,
    slot_y: u32,
    slot_width: u32,
    slot_height: u32,
    has_alpha: bool,
    config: &Config,
) -> Vec<bool> {
    let mut mask = vec![false; (slot_width * slot_height) as usize];

    if uses_transparency(has_alpha, config.background_mode) {
        for y in 0..slot_height {
            for x in 0..slot_width {
                let pixel = source.get_pixel(slot_x + x, slot_y + y);
                mask[index_of(x, y, slot_width)] = pixel[3] <= config.alpha_threshold;
            }
        }
        return mask;
    }

    let mut queue = VecDeque::new();
    for x in 0..slot_width {
        try_push_background(
            source, slot_x, slot_y, slot_width, x, 0, config, &mut mask, &mut queue,
        );
        try_push_background(
            source,
            slot_x,
            slot_y,
            slot_width,
            x,
            slot_height - 1,
            config,
            &mut mask,
            &mut queue,
        );
    }
    for y in 0..slot_height {
        try_push_background(
            source, slot_x, slot_y, slot_width, 0, y, config, &mut mask, &mut queue,
        );
        try_push_background(
            source,
            slot_x,
            slot_y,
            slot_width,
            slot_width - 1,
            y,
            config,
            &mut mask,
            &mut queue,
        );
    }

    while let Some((x, y)) = queue.pop_front() {
        if x > 0 {
            try_push_background(
                source,
                slot_x,
                slot_y,
                slot_width,
                x - 1,
                y,
                config,
                &mut mask,
                &mut queue,
            );
        }
        if x + 1 < slot_width {
            try_push_background(
                source,
                slot_x,
                slot_y,
                slot_width,
                x + 1,
                y,
                config,
                &mut mask,
                &mut queue,
            );
        }
        if y > 0 {
            try_push_background(
                source,
                slot_x,
                slot_y,
                slot_width,
                x,
                y - 1,
                config,
                &mut mask,
                &mut queue,
            );
        }
        if y + 1 < slot_height {
            try_push_background(
                source,
                slot_x,
                slot_y,
                slot_width,
                x,
                y + 1,
                config,
                &mut mask,
                &mut queue,
            );
        }
    }

    mask
}

#[allow(clippy::too_many_arguments)]
fn try_push_background(
    source: &RgbaImage,
    slot_x: u32,
    slot_y: u32,
    slot_width: u32,
    x: u32,
    y: u32,
    config: &Config,
    mask: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
) {
    let index = index_of(x, y, slot_width);
    if mask[index] {
        return;
    }

    let pixel = source.get_pixel(slot_x + x, slot_y + y);
    if is_white_background_candidate(pixel, config) {
        mask[index] = true;
        queue.push_back((x, y));
    }
}

fn uses_transparency(has_alpha: bool, background_mode: BackgroundMode) -> bool {
    background_mode == BackgroundMode::Transparent
        || (background_mode == BackgroundMode::Auto && has_alpha)
}

fn is_white_background_candidate(pixel: &Rgba<u8>, config: &Config) -> bool {
    if pixel[3] <= config.alpha_threshold {
        return true;
    }

    let red = pixel[0];
    let green = pixel[1];
    let blue = pixel[2];
    let max_channel = red.max(green).max(blue);
    let min_channel = red.min(green).min(blue);

    min_channel >= config.white_threshold && max_channel - min_channel <= config.white_chroma
}

fn index_of(x: u32, y: u32, width: u32) -> usize {
    (y * width + x) as usize
}

fn div_ceil(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

fn next_string(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &str,
) -> Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("{name} requires a value"))
}

fn next_path(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &str,
) -> Result<PathBuf> {
    Ok(PathBuf::from(next_string(args, name)?))
}

fn next_u32(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &str,
) -> Result<u32> {
    let value = next_string(args, name)?;
    value
        .parse()
        .with_context(|| format!("{name} must be an unsigned integer"))
}

fn next_u8(args: &mut std::iter::Peekable<impl Iterator<Item = String>>, name: &str) -> Result<u8> {
    let value = next_string(args, name)?;
    value
        .parse()
        .with_context(|| format!("{name} must be between 0 and 255"))
}

fn parse_size(value: &str) -> Result<Size> {
    let normalized = value.replace(['X', ',', '*'], "x");
    if let Some((width, height)) = normalized.split_once('x') {
        let width = parse_size_component(width, "width")?;
        let height = parse_size_component(height, "height")?;
        return Ok(Size { width, height });
    }

    let side = parse_size_component(&normalized, "size")?;
    Ok(Size {
        width: side,
        height: side,
    })
}

fn parse_size_component(value: &str, name: &str) -> Result<u32> {
    let parsed = value
        .trim()
        .parse()
        .with_context(|| format!("cell {name} must be an unsigned integer"))?;
    if parsed == 0 {
        bail!("cell {name} must be greater than zero");
    }
    Ok(parsed)
}

fn print_help() {
    println!(
        r#"sprite_sheet_compactor

Trim oversized gaps from generated sprite sheets and repack frames into compact,
uniform cells. Frames are bottom-center aligned by default.

Usage:
  cargo run -p sprite_sheet_compactor -- \
    --input raw.png \
    --output compact.png \
    --frames 4 \
    --input-columns 4 \
    --padding 8 \
    --cell-size 64x64

Options:
  -i, --input <path>             Source PNG sprite sheet.
  -o, --output <path>            Output PNG sprite sheet.
  -f, --frames <count>           Number of frames to process.
      --input-columns <count>    Source grid columns. Defaults to frame count.
      --input-rows <count>       Source grid rows. Defaults to ceil(frames / columns).
      --output-columns <count>   Output grid columns. Defaults to input columns.
      --cell-size <WxH>          Output cell size, for example 64x64. Also accepts 64.
      --frame-size <WxH>         Alias for --cell-size.
  -p, --padding <pixels>         Padding inside each output cell. Defaults to 4.
      --filter <name>            nearest, triangle, catmullrom, gaussian, or lanczos3.
                                  Defaults to nearest for pixel-art style sheets.
      --background <mode>        auto, transparent, or white. Defaults to auto.
      --alpha-threshold <0-255>  Alpha value treated as transparent. Defaults to 8.
      --white-threshold <0-255>  Minimum RGB channel for white background. Defaults to 225.
      --white-chroma <0-255>     Max RGB spread for white background. Defaults to 28.
      --preserve-canvas          Clear background but keep each input frame canvas size.
      --fill-transparent         Fill transparent pixels from nearest visible pixels.
      --stretch-to-cell          Stretch each trimmed frame to fill --cell-size exactly.
      --keep-background          Keep detected edge background pixels instead of clearing them.
  -h, --help                     Show this help.
"#
    );
}
