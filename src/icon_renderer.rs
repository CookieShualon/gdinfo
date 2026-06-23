use std::{fs, io::Cursor, path::PathBuf, time::Duration};

use eframe::egui::{ColorImage, TextureHandle};
use image::ImageFormat;
use reqwest::Client;

use crate::models::PlayerIcon;

const ICONKIT_BASE_URL: &str = "https://gdbrowser.com/icon";

pub async fn load_icon_image(icon: &PlayerIcon) -> Result<ColorImage, IconError> {
    if icon.cube_id.trim().is_empty()
        || icon.primary_color.trim().is_empty()
        || icon.secondary_color.trim().is_empty()
    {
        return Err(IconError::InvalidIconData);
    }

    let path = cache_path(icon);
    let bytes = if path.exists() {
        fs::read(&path).map_err(IconError::Io)?
    } else {
        let bytes = download_icon(icon).await?;
        let bytes = recolor_icon(&bytes, icon)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(IconError::Io)?;
        }
        fs::write(&path, &bytes).map_err(IconError::Io)?;
        bytes
    };

    decode_image(&bytes)
}

pub fn texture_from_image(ctx: &eframe::egui::Context, image: ColorImage) -> TextureHandle {
    ctx.load_texture("player-icon", image, Default::default())
}

fn build_icon_url(icon: &PlayerIcon) -> String {
    format!(
        "{}/preview?icon={cube}&form=icon",
        ICONKIT_BASE_URL,
        cube = icon.cube_id.trim(),
    )
}

async fn download_icon(icon: &PlayerIcon) -> Result<Vec<u8>, IconError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(IconError::Http)?;

    let response = client
        .get(build_icon_url(icon))
        .send()
        .await
        .map_err(IconError::Http)?;

    if !response.status().is_success() {
        return Err(IconError::HttpStatus(response.status().as_u16()));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(IconError::Http)
}

fn decode_image(bytes: &[u8]) -> Result<ColorImage, IconError> {
    let image = image::load_from_memory(bytes)
        .map_err(IconError::Image)?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];

    Ok(ColorImage::from_rgba_unmultiplied(size, image.as_raw()))
}

fn recolor_icon(bytes: &[u8], icon: &PlayerIcon) -> Result<Vec<u8>, IconError> {
    let mut image = image::load_from_memory(bytes)
        .map_err(IconError::Image)?
        .to_rgba8();
    let primary = gd_color(&icon.primary_color).unwrap_or([175, 175, 175]);
    let secondary = gd_color(&icon.secondary_color).unwrap_or([255, 255, 255]);

    for pixel in image.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 || (r < 20 && g < 20 && b < 20) {
            continue;
        }

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        if max.saturating_sub(min) > 18 {
            continue;
        }

        let target = if max > 225 { secondary } else { primary };
        let base = if max > 225 { 255.0 } else { 175.0 };
        let shade = (max as f32 / base).clamp(0.0, 1.35);

        pixel.0 = [
            shaded_channel(target[0], shade),
            shaded_channel(target[1], shade),
            shaded_channel(target[2], shade),
            a,
        ];
    }

    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(IconError::Image)?;
    Ok(output.into_inner())
}

fn shaded_channel(channel: u8, shade: f32) -> u8 {
    ((channel as f32 * shade).round() as i32).clamp(0, 255) as u8
}

fn cache_path(icon: &PlayerIcon) -> PathBuf {
    PathBuf::from("cache").join("icons").join(format!(
        "v2_cube_{}_{}_{}_{}.png",
        icon.cube_id.trim(),
        icon.primary_color.trim(),
        icon.secondary_color.trim(),
        if icon.glow_enabled { "glow" } else { "noglow" }
    ))
}

fn gd_color(color_id: &str) -> Option<[u8; 3]> {
    let color = match color_id.trim().parse::<u16>().ok()? {
        0 => [125, 255, 0],
        1 => [0, 255, 0],
        2 => [0, 255, 125],
        3 => [0, 255, 255],
        4 => [0, 125, 255],
        5 => [0, 0, 255],
        6 => [125, 0, 255],
        7 => [255, 0, 255],
        8 => [255, 0, 125],
        9 => [255, 0, 0],
        10 => [255, 125, 0],
        11 => [255, 255, 0],
        12 => [255, 255, 255],
        13 => [185, 0, 255],
        14 => [255, 185, 0],
        15 => [0, 0, 0],
        16 => [0, 200, 255],
        17 => [175, 175, 175],
        18 => [90, 90, 90],
        19 => [255, 125, 125],
        20 => [0, 175, 75],
        21 => [0, 125, 125],
        22 => [0, 75, 175],
        23 => [75, 0, 175],
        24 => [125, 0, 125],
        25 => [175, 0, 75],
        26 => [175, 75, 0],
        27 => [125, 125, 0],
        28 => [75, 175, 0],
        29 => [255, 75, 0],
        30 => [150, 50, 0],
        31 => [150, 100, 0],
        32 => [100, 150, 0],
        33 => [0, 150, 100],
        34 => [0, 100, 150],
        35 => [100, 0, 150],
        36 => [150, 0, 100],
        37 => [150, 0, 0],
        38 => [0, 150, 0],
        39 => [0, 0, 150],
        40 => [125, 255, 175],
        41 => [125, 125, 255],
        42 => [255, 250, 127],
        43 => [250, 127, 255],
        44 => [0, 255, 192],
        45 => [80, 50, 14],
        46 => [205, 165, 118],
        47 => [182, 128, 255],
        48 => [255, 58, 58],
        49 => [77, 77, 143],
        50 => [0, 10, 76],
        51 => [253, 212, 206],
        52 => [190, 181, 255],
        53 => [112, 0, 0],
        54 => [82, 2, 0],
        55 => [56, 1, 6],
        56 => [128, 79, 79],
        57 => [122, 53, 53],
        58 => [81, 36, 36],
        59 => [163, 98, 70],
        60 => [117, 73, 54],
        61 => [86, 53, 40],
        62 => [255, 185, 114],
        63 => [255, 160, 64],
        64 => [102, 49, 30],
        65 => [91, 39, 0],
        66 => [71, 32, 0],
        67 => [167, 123, 77],
        68 => [109, 83, 57],
        69 => [81, 62, 42],
        70 => [255, 255, 192],
        71 => [253, 224, 160],
        72 => [192, 255, 160],
        73 => [177, 255, 109],
        74 => [192, 255, 224],
        75 => [148, 255, 228],
        76 => [67, 161, 138],
        77 => [49, 109, 95],
        78 => [38, 84, 73],
        79 => [0, 96, 0],
        80 => [0, 64, 0],
        81 => [0, 96, 96],
        82 => [0, 64, 64],
        83 => [160, 255, 255],
        84 => [1, 7, 112],
        85 => [0, 73, 109],
        86 => [0, 50, 76],
        87 => [0, 38, 56],
        88 => [80, 128, 173],
        89 => [51, 83, 117],
        90 => [35, 60, 86],
        91 => [224, 224, 224],
        92 => [61, 6, 140],
        93 => [55, 8, 96],
        94 => [64, 64, 64],
        95 => [111, 73, 164],
        96 => [84, 54, 127],
        97 => [66, 42, 99],
        98 => [252, 181, 255],
        99 => [175, 87, 175],
        100 => [130, 67, 130],
        101 => [94, 49, 94],
        102 => [128, 128, 128],
        103 => [102, 3, 62],
        104 => [71, 1, 52],
        105 => [210, 255, 50],
        106 => [118, 189, 255],
        _ => return None,
    };

    Some(color)
}

#[derive(Debug)]
pub enum IconError {
    InvalidIconData,
    Http(reqwest::Error),
    HttpStatus(u16),
    Io(std::io::Error),
    Image(image::ImageError),
}

impl std::fmt::Display for IconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIconData => write!(f, "missing icon data"),
            Self::Http(error) if error.is_timeout() => write!(f, "icon request timed out"),
            Self::Http(error) => write!(f, "icon network error: {error}"),
            Self::HttpStatus(status) => write!(f, "icon request returned HTTP {status}"),
            Self::Io(error) => write!(f, "icon cache error: {error}"),
            Self::Image(error) => write!(f, "icon decode error: {error}"),
        }
    }
}

impl std::error::Error for IconError {}
