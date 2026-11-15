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

#[derive(Clone, Copy)]
struct V3 { x: f32, y: f32, z: f32 }

fn rot(v: V3, yaw: f32, pitch: f32) -> V3 {
    let (sy, cy) = yaw.sin_cos();
    let x1 = v.x * cy + v.z * sy;
    let z1 = -v.x * sy + v.z * cy;
    let (sp, cp) = pitch.sin_cos();
    let y2 = v.y * cp - z1 * sp;
    let z2 = v.y * sp + z1 * cp;
    V3 { x: x1, y: y2, z: z2 }
}

fn draw_quad(
    img: &image::DynamicImage,
    fb: &mut RgbaImage,
    zb: &mut [f32],
    pts: [V3; 4],
    uv: (u32, u32, u32, u32),
    shade: f32,
    yaw: f32,
    pitch: f32,
) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;
    let proj = |v: V3| -> (f32, f32, f32) {
        let r = rot(v, yaw, pitch);
        let z = r.z + 48.0;
        let s = 280.0 / z.max(1.0);
        (w as f32 * 0.5 + r.x * s, h as f32 * 0.5 - r.y * s, z)
    };
    let p = [proj(pts[0]), proj(pts[1]), proj(pts[2]), proj(pts[3])];
    let tris = [
        (p[0], p[1], p[2], (0.0, 0.0), (1.0, 0.0), (1.0, 1.0)),
        (p[0], p[2], p[3], (0.0, 0.0), (1.0, 1.0), (0.0, 1.0)),
    ];

    for (a, b, c, uva, uvb, uvc) in tris {
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        if area.abs() < 0.01 { continue; }
        let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
        let max_x = a.0.max(b.0).max(c.0).ceil().min((w - 1) as f32) as i32;
        let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
        let max_y = a.1.max(b.1).max(c.1).ceil().min((h - 1) as f32) as i32;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let fx = px as f32 + 0.5;
                let fy = py as f32 + 0.5;
                let w0 = ((b.0 - fx) * (c.1 - fy) - (b.1 - fy) * (c.0 - fx)) / area;
                let w1 = ((c.0 - fx) * (a.1 - fy) - (c.1 - fy) * (a.0 - fx)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 { continue; }

                let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                let idx = (py * w + px) as usize;
                if z >= zb[idx] { continue; }

                let u = (w0 * uva.0 + w1 * uvb.0 + w2 * uvc.0).clamp(0.0, 0.999);
                let v = (w0 * uva.1 + w1 * uvb.1 + w2 * uvc.1).clamp(0.0, 0.999);
                let tx = uv.0 + (u * uv.2 as f32) as u32;
                let ty = uv.1 + (v * uv.3 as f32) as u32;
                let pix = img.get_pixel(tx, ty);
                if pix[3] < 128 { continue; }

                zb[idx] = z;
                fb.put_pixel(px as u32, py as u32, image::Rgba([
                    (pix[0] as f32 * shade) as u8,
                    (pix[1] as f32 * shade) as u8,
                    (pix[2] as f32 * shade) as u8,
                    255,
                ]));
            }
        }
    }
}

fn draw_box(
    img: &image::DynamicImage,
    fb: &mut RgbaImage,
    zb: &mut [f32],
    c: (f32, f32, f32),
    sz: (f32, f32, f32),
    faces: [(u32, u32, u32, u32); 6], // front, back, left, right, top, bottom
    yaw: f32,
    pitch: f32,
) {
    let (hx, hy, hz) = (sz.0 * 0.5, sz.1 * 0.5, sz.2 * 0.5);
    let v = |dx: f32, dy: f32, dz: f32| V3 { x: c.0 + dx * hx, y: c.1 + dy * hy, z: c.2 + dz * hz };
    let quads = [
        ([v(-1.0, 1.0, 1.0), v(1.0, 1.0, 1.0), v(1.0, -1.0, 1.0), v(-1.0, -1.0, 1.0)], faces[0], 1.00),
        ([v(1.0, 1.0, -1.0), v(-1.0, 1.0, -1.0), v(-1.0, -1.0, -1.0), v(1.0, -1.0, -1.0)], faces[1], 0.80),
        ([v(-1.0, 1.0, -1.0), v(-1.0, 1.0, 1.0), v(-1.0, -1.0, 1.0), v(-1.0, -1.0, -1.0)], faces[2], 0.88),
        ([v(1.0, 1.0, 1.0), v(1.0, 1.0, -1.0), v(1.0, -1.0, -1.0), v(1.0, -1.0, 1.0)], faces[3], 0.88),
        ([v(-1.0, 1.0, -1.0), v(1.0, 1.0, -1.0), v(1.0, 1.0, 1.0), v(-1.0, 1.0, 1.0)], faces[4], 1.00),
        ([v(-1.0, -1.0, 1.0), v(1.0, -1.0, 1.0), v(1.0, -1.0, -1.0), v(-1.0, -1.0, -1.0)], faces[5], 0.70),
    ];
    for (pts, uv, shade) in quads {
        draw_quad(img, fb, zb, pts, uv, shade, yaw, pitch);
    }
}

pub fn render_3d(img: &image::DynamicImage, yaw: f32, pitch: f32) -> slint::Image {
    if img.width() < 64 || img.height() < 32 {
        return render_preview(img);
    }
    let (w, h) = (180u32, 260u32);
    let mut fb = RgbaImage::new(w, h);
    let mut zb = vec![f32::INFINITY; (w * h) as usize];
    let m = img.height() >= 64;

    // Head, Body
    draw_box(img, &mut fb, &mut zb, (0.0, 12.0, 0.0), (8.0, 8.0, 8.0), [(8,8,8,8), (24,8,8,8), (0,8,8,8), (16,8,8,8), (8,0,8,8), (16,0,8,8)], yaw, pitch);
    draw_box(img, &mut fb, &mut zb, (0.0, 2.0, 0.0), (8.0, 12.0, 4.0), [(20,20,8,12), (32,20,8,12), (16,20,4,12), (28,20,4,12), (20,16,8,4), (28,16,8,4)], yaw, pitch);
    // Right & Left Arm
    draw_box(img, &mut fb, &mut zb, (-6.0, 2.0, 0.0), (4.0, 12.0, 4.0), [(44,20,4,12), (52,20,4,12), (40,20,4,12), (48,20,4,12), (44,16,4,4), (48,16,4,4)], yaw, pitch);
    let la = if m { [(36,52,4,12), (44,52,4,12), (32,52,4,12), (40,52,4,12), (36,48,4,4), (40,48,4,4)] }
             else { [(44,20,4,12), (52,20,4,12), (40,20,4,12), (48,20,4,12), (44,16,4,4), (48,16,4,4)] };
    draw_box(img, &mut fb, &mut zb, (6.0, 2.0, 0.0), (4.0, 12.0, 4.0), la, yaw, pitch);
    // Right & Left Leg
    draw_box(img, &mut fb, &mut zb, (-2.0, -10.0, 0.0), (4.0, 12.0, 4.0), [(4,20,4,12), (12,20,4,12), (0,20,4,12), (8,20,4,12), (4,16,4,4), (8,16,4,4)], yaw, pitch);
    let ll = if m { [(20,52,4,12), (28,52,4,12), (16,52,4,12), (24,52,4,12), (20,48,4,4), (24,48,4,4)] }
             else { [(4,20,4,12), (12,20,4,12), (0,20,4,12), (8,20,4,12), (4,16,4,4), (8,16,4,4)] };
    draw_box(img, &mut fb, &mut zb, (2.0, -10.0, 0.0), (4.0, 12.0, 4.0), ll, yaw, pitch);

    let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(fb.as_raw(), w, h);
    slint::Image::from_rgba8(buf)
}