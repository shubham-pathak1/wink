use std::{env, fs::File, path::PathBuf};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::imageops::FilterType;

fn main() {
    const SIZES: [u32; 5] = [16, 20, 24, 32, 48];

    println!("cargo:rerun-if-changed=assets/wink_icon.png");

    let source = image::open("assets/wink_icon.png")
        .expect("assets/wink_icon.png must be a readable PNG")
        .to_rgba8();
    let source = crop_visible_artwork(&source);
    let mut icon = IconDir::new(ResourceType::Icon);

    for size in SIZES {
        let pixels = image::imageops::resize(&source, size, size, FilterType::Lanczos3).into_raw();
        let image = IconImage::from_rgba_data(size, size, pixels);
        let entry = IconDirEntry::encode_as_bmp(&image).expect("failed to encode icon image");
        icon.add_entry(entry);
    }

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo")).join("wink.ico");
    icon.write(File::create(output).expect("failed to create generated icon"))
        .expect("failed to write generated icon");
    icon.write(File::create("assets/wink.ico").expect("failed to create installer icon"))
        .expect("failed to write installer icon");

    let mut resources = winresource::WindowsResource::new();
    resources.set_icon("assets/wink.ico");
    resources
        .compile()
        .expect("failed to embed executable icon");
}

fn crop_visible_artwork(source: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = source.dimensions();
    let (mut min_x, mut min_y) = (width, height);
    let (mut max_x, mut max_y) = (0, 0);

    for (x, y, pixel) in source.enumerate_pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha > 0 && red.max(green).max(blue) > 24 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if min_x > max_x || min_y > max_y {
        return source.clone();
    }

    let artwork_width = max_x - min_x + 1;
    let artwork_height = max_y - min_y + 1;
    let side =
        ((artwork_width.max(artwork_height) as f32 * 1.12).ceil() as u32).min(width.min(height));
    let center_x = min_x + artwork_width / 2;
    let center_y = min_y + artwork_height / 2;
    let left = center_x.saturating_sub(side / 2).min(width - side);
    let top = center_y.saturating_sub(side / 2).min(height - side);

    image::imageops::crop_imm(source, left, top, side, side).to_image()
}
