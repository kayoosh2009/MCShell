use image::{imageops, GenericImageView, RgbaImage};
use slint::{Rgba8Pixel, SharedPixelBuffer};

fn blit(dst: &mut RgbaImage, src: &image::DynamicImage, sx: u32, sy: u32, w: u32, h: u32, dx: i64, dy: i64, flip: bool) {
    let mut part = src.view(sx, sy, w, h).to_image();
    if flip {
        part = imageops::flip_horizontal(&part);
    }
    imageops::overlay(dst, &part, dx, dy);
}

pub fn render_preview(img: &image::DynamicImage) -> slint::Image {
    if img.width() < 64 || img.height() < 32 {
        let rgba = img.to_rgba8();
        let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height());
        return slint::Image::from_rgba8(buf);
    }

    let mut canvas = RgbaImage::new(16, 32);
    let modern = img.height() >= 64;

    // Голова и шляпа
    blit(&mut canvas, img, 8, 8, 8, 8, 4, 0, false);
    blit(&mut canvas, img, 40, 8, 8, 8, 4, 0, false);

    // Торс и куртка
    blit(&mut canvas, img, 20, 20, 8, 12, 4, 8, false);
    if modern {
        blit(&mut canvas, img, 20, 36, 8, 12, 4, 8, false);
    }

    // Правая и левая руки
    blit(&mut canvas, img, 44, 20, 4, 12, 0, 8, false);
    if modern {
        blit(&mut canvas, img, 36, 52, 4, 12, 12, 8, false);
    } else {
        blit(&mut canvas, img, 44, 20, 4, 12, 12, 8, true);
    }

    // Правая и левая ноги
    blit(&mut canvas, img, 4, 20, 4, 12, 4, 20, false);
    if modern {
        blit(&mut canvas, img, 20, 52, 4, 12, 8, 20, false);
    } else {
        blit(&mut canvas, img, 4, 20, 4, 12, 8, 20, true);
    }

    let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(canvas.as_raw(), canvas.width(), canvas.height());
    slint::Image::from_rgba8(buf)
}