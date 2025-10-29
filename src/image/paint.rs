use anyhow::Result;
use image::imageops::overlay;
use image::{DynamicImage, ImageBuffer, Rgba};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_rect_mut, draw_line_segment_mut,
};
use imageproc::rect::Rect;

use crate::core::punchcards::PunchCard;

const CARD_WIDTH_IN: f32 = 7.375;
const CARD_HEIGHT_IN: f32 = 3.25;
const A4_WIDTH_IN: f32 = 8.27;
const A4_HEIGHT_IN: f32 = 11.69;
pub const GLYPH_WIDTH: usize = 5;
pub const GLYPH_HEIGHT: usize = 7;
const ROW_BIT_ORDER: [usize; 12] = [11, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

/// Visual styles for PNG rendering.
#[derive(Debug, Clone, Copy)]
pub enum CardImageStyle {
    Plain,
    Interpreter,
    Keypunch,
}

/// Target layout for the generated image.
#[derive(Debug, Clone, Copy)]
pub enum PageLayout {
    Card,
    A4,
}

/// Options controlling PNG generation.
#[derive(Debug, Clone, Copy)]
pub struct ImageRenderOptions {
    pub style: CardImageStyle,
    pub dpi: u32,
    pub layout: PageLayout,
}

struct Palette {
    card_bg: Rgba<u8>,
    page_bg: Rgba<u8>,
    grid: Rgba<u8>,
    hole: Rgba<u8>,
    text: Rgba<u8>,
    border: Rgba<u8>,
    header: Option<Rgba<u8>>,
}

/// Render a punch card into a PNG image using the supplied options.
pub fn render_card_image(card: &PunchCard, options: &ImageRenderOptions) -> Result<DynamicImage> {
    let dpi = options.dpi.clamp(72, 1200);
    let palette = palette(options.style, matches!(options.layout, PageLayout::Card));

    let card_width_px = inches_to_px(CARD_WIDTH_IN, dpi);
    let card_height_px = inches_to_px(CARD_HEIGHT_IN, dpi);
    let dpi_f = dpi as f32;

    let margin_x = (0.18 * dpi_f).round() as i32;
    let margin_top = (0.55 * dpi_f).round() as i32;
    let margin_bottom = (0.35 * dpi_f).round() as i32;

    let mut card_img =
        ImageBuffer::from_pixel(card_width_px, card_height_px, palette.card_bg.clone());

    if let Some(header_color) = palette.header {
        let header_height = (0.4 * dpi_f).round() as u32;
        draw_filled_rect_mut(
            &mut card_img,
            Rect::at(0, 0).of_size(card_width_px, header_height.min(card_height_px)),
            header_color,
        );
    }

    let border_rect = Rect::at(0, 0).of_size(card_width_px, card_height_px);
    draw_hollow_rect_mut(&mut card_img, border_rect, palette.border);

    let col_count = card.columns().len();
    let col_spacing =
        (card_width_px as f32 - 2.0 * margin_x as f32).max(1.0) / (col_count as f32 - 1.0);
    let row_spacing = (card_height_px as f32 - (margin_top + margin_bottom) as f32).max(1.0)
        / (ROW_BIT_ORDER.len() as f32 - 1.0);
    let hole_radius = (col_spacing.min(row_spacing) * 0.2).round() as i32;
    let hole_radius = hole_radius.max(2);

    for col in 0..=col_count {
        if col == 0 || col == col_count || col % 10 == 0 {
            let x = margin_x as f32 + col as f32 * col_spacing;
            draw_line_segment_mut(
                &mut card_img,
                (x, margin_top as f32),
                (x, (card_height_px as i32 - margin_bottom) as f32),
                palette.grid,
            );
        }
    }

    for (col_idx, cell) in card.columns().iter().enumerate() {
        let center_x = (margin_x as f32 + col_idx as f32 * col_spacing).round() as i32;
        for (row_idx, bit) in ROW_BIT_ORDER.iter().enumerate() {
            if (cell.0 >> bit) & 1 == 1 {
                let center_y = (margin_top as f32 + row_idx as f32 * row_spacing).round() as i32;
                draw_filled_circle_mut(
                    &mut card_img,
                    (center_x, center_y),
                    hole_radius,
                    palette.hole,
                );
            }
        }
    }

    let scale = (dpi_f / 120.0).ceil() as u32;
    let scale = scale.max(2);
    let glyph_half_width = ((GLYPH_WIDTH as u32 * scale) as f32 / 2.0).round() as i32;
    let text_baseline = (margin_top as f32 - row_spacing * 0.85).round() as i32;
    for (col_idx, ch) in card.text().iter().enumerate() {
        let center_x = (margin_x as f32 + col_idx as f32 * col_spacing).round() as i32;
        let glyph_x = center_x - glyph_half_width;
        draw_glyph(
            &mut card_img,
            glyph_x,
            text_baseline,
            *ch,
            palette.text,
            scale,
        );
    }

    let final_image = match options.layout {
        PageLayout::Card => DynamicImage::ImageRgba8(card_img),
        PageLayout::A4 => {
            let page_width = inches_to_px(A4_WIDTH_IN, dpi);
            let page_height = inches_to_px(A4_HEIGHT_IN, dpi);
            let mut page =
                ImageBuffer::from_pixel(page_width, page_height, palette.page_bg.clone());
            let offset_x = ((page_width as i32 - card_width_px as i32) / 2).max(0);
            let offset_y = ((page_height as i32 - card_height_px as i32) / 2).max(0);
            overlay(&mut page, &card_img, offset_x as i64, offset_y as i64);
            DynamicImage::ImageRgba8(page)
        }
    };

    Ok(final_image)
}

fn inches_to_px(inches: f32, dpi: u32) -> u32 {
    (inches * dpi as f32).round() as u32
}

fn palette(style: CardImageStyle, card_only: bool) -> Palette {
    match style {
        CardImageStyle::Plain => Palette {
            card_bg: rgba(0xf4, 0xe8, 0xcc, 0xff),
            page_bg: if card_only {
                rgba(0xf4, 0xe8, 0xcc, 0xff)
            } else {
                rgba(0xfd, 0xfa, 0xf3, 0xff)
            },
            grid: rgba(0xd7, 0xc9, 0xa8, 0xff),
            hole: rgba(0x28, 0x24, 0x1f, 0xff),
            text: rgba(0x28, 0x24, 0x1f, 0xff),
            border: rgba(0x7d, 0x6b, 0x54, 0xff),
            header: None,
        },
        CardImageStyle::Interpreter => Palette {
            card_bg: rgba(0xf6, 0xe3, 0xc6, 0xff),
            page_bg: if card_only {
                rgba(0xf6, 0xe3, 0xc6, 0xff)
            } else {
                rgba(0xfc, 0xf7, 0xef, 0xff)
            },
            grid: rgba(0xd1, 0xba, 0x9b, 0xff),
            hole: rgba(0x24, 0x22, 0x1d, 0xff),
            text: rgba(0x1f, 0x1b, 0x14, 0xff),
            border: rgba(0x86, 0x74, 0x5d, 0xff),
            header: Some(rgba(0xe6, 0xcb, 0xa6, 0xff)),
        },
        CardImageStyle::Keypunch => Palette {
            card_bg: rgba(0xf5, 0xd7, 0xb5, 0xff),
            page_bg: if card_only {
                rgba(0xf5, 0xd7, 0xb5, 0xff)
            } else {
                rgba(0xfa, 0xf2, 0xe7, 0xff)
            },
            grid: rgba(0xca, 0xa0, 0x79, 0xff),
            hole: rgba(0x2b, 0x21, 0x1d, 0xff),
            text: rgba(0x21, 0x18, 0x15, 0xff),
            border: rgba(0x82, 0x63, 0x4d, 0xff),
            header: Some(rgba(0xe6, 0xb8, 0x8f, 0xff)),
        },
    }
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba<u8> {
    Rgba([r, g, b, a])
}

fn draw_glyph(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    ch: char,
    color: Rgba<u8>,
    scale: u32,
) {
    let pattern = glyph_pattern(ch);
    for (row, bits) in pattern.iter().enumerate() {
        for col in 0..GLYPH_WIDTH {
            if bits & (1 << (GLYPH_WIDTH - 1 - col)) != 0 {
                let px = x + (col as i32 * scale as i32);
                let py = y + (row as i32 * scale as i32);
                draw_filled_rect_mut(image, Rect::at(px, py).of_size(scale, scale), color);
            }
        }
    }
}

#[rustfmt::skip]
fn glyph_pattern(ch: char) -> [u8; GLYPH_HEIGHT] {
    match ch.to_ascii_uppercase() {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '/' => [0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000, 0b00000],
        ':' => [0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000],
        '#' => [0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b01010],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110],
        '\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '=' => [0b00000, 0b11111, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000],
        '"' => [0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00110],
        ',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '+' => [0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000, 0b00000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '$' => [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100],
        '%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111, 0b00000],
        '|' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        ';' => [0b00000, 0b00100, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000],
        '*' => [0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000, 0b00000],
        '?' => [0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    }
}
