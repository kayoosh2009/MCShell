use image::GenericImageView;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Конвертирует PNG скина в список Line'ов для ratatui.
/// width/height — размер в символах (height в 2 раза меньше пиксельной высоты).
pub fn skin_to_lines(img: &image::DynamicImage, width: u32, height: u32) -> Vec<Line<'static>> {
    // Увеличиваем высоту в 2 раза, так как один символ "▀" занимает 2 пикселя по вертикали
    let resized = img.resize_exact(width, height * 2, image::imageops::FilterType::Nearest);
    let mut lines = Vec::with_capacity(height as usize);

    for y in (0..height * 2).step_by(2) {
        let mut spans = Vec::with_capacity(width as usize);
        for x in 0..width {
            let top = resized.get_pixel(x, y);
            let bot = resized.get_pixel(x, y + 1);

            // Функция для конвертации RGBA в Color, учитывая прозрачность
            let to_color = |p: image::Rgba<u8>| -> Color {
                if p[3] < 128 {
                    Color::Reset // Прозрачный пиксель (оставляет фон терминала)
                } else {
                    Color::Rgb(p[0], p[1], p[2])
                }
            };

            spans.push(Span::styled("▀", Style::new().fg(to_color(top)).bg(to_color(bot))));
        }
        lines.push(Line::from(spans));
    }
    lines
}