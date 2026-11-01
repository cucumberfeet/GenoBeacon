//! GenoBeacon device screen.
//!
//! The Mix and Mix Create panels are 800×480. Pieces are JPEG, which is the
//! format the device protocol already expects, at the same coordinates the
//! hardware buttons and dials line up with. Every pixel is drawn here from
//! the GenoCorp palette (black, red, blue) and the OFL Tinos italic wordmark.
//! BEACN's own tiles, logos and screen kit are not loaded.

use crate::integrations::pipeweaver::helpers::Mix;
use anyhow::Result;
use fontdue::Font;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, Rgb, RgbImage, Rgba, RgbaImage, imageops};
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

pub type Jpeg = Arc<Vec<u8>>;

// ---- Geometry (absolute screen pixels, 800x480). These match the Mix Create's
// physical dials and on-screen buttons; the artwork inside them is ours. ----
#[allow(dead_code)]
pub const SCREEN: (u32, u32) = (800, 480);
pub const CARD_X: [u32; 4] = [12, 208, 405, 601];
pub const CARD_Y: u32 = 111;
pub const CARD_W: u32 = 187;
pub const CARD_H: u32 = 276;
pub const TILE_H: u32 = 180;
const BAND_Y: u32 = TILE_H;
const BAND_H: u32 = 36;
const LINE_Y: u32 = BAND_Y + BAND_H;
const LINE_H: u32 = 3;
const SUB_LINE_CENTRES: [u32; 2] = [240, 264];
pub const BUTTON_W: u32 = 105;
pub const BUTTON_H: u32 = 32;
pub const BUTTON_Y: u32 = 433;
pub const PILL_POS: (u32, u32) = (371, 69);
pub const PILL_SIZE: (u32, u32) = (58, 32);
pub const DOTS_POS: (u32, u32) = (300, 398);
pub const DOTS_SIZE: (u32, u32) = (200, 24);
pub const STATUS_POS: (u32, u32) = (0, 392);
const STATUS_SIZE: (u32, u32) = (800, 40);
pub const DIAL: (u32, u32) = (168, 168);

pub fn button_x(slot: usize) -> u32 {
    CARD_X[slot] + (CARD_W - BUTTON_W) / 2
}

pub fn dial_offset() -> (i32, i32) {
    (
        (CARD_W as i32 - DIAL.0 as i32) / 2,
        (TILE_H as i32 - DIAL.1 as i32) / 2,
    )
}

// GenoCorp palette. Not sampled from BEACN.
const BG: Rgba<u8> = Rgba([7, 8, 12, 255]);
const PANEL: Rgba<u8> = Rgba([16, 18, 24, 255]);
const RED: Rgba<u8> = Rgba([225, 6, 0, 255]);
const BLUE: Rgba<u8> = Rgba([61, 126, 255, 255]);
const PAPER: Rgba<u8> = Rgba([232, 234, 240, 255]);
const DIM: Rgba<u8> = Rgba([150, 156, 172, 255]);
const TRACK: Rgba<u8> = Rgba([38, 44, 58, 255]);
const METER: Rgba<u8> = Rgba([190, 210, 255, 255]);

static TINOS_ITALIC: &[u8] = include_bytes!("../../../resources/fonts/tinos/Tinos-Italic.ttf");
static TINOS_BOLD: &[u8] = include_bytes!("../../../resources/fonts/tinos/Tinos-Bold.ttf");

const LABEL_SIZE: f32 = 16.0;
const SUB_SIZE: f32 = 13.5;
const BUTTON_SIZE: f32 = 13.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // Monitor/Chat/Media faces are drawn; the mixer only selects Volume, Muted and StreamMuted today.
pub enum Face {
    Volume = 0,
    Muted = 1,
    StreamMuted = 2,
    MonitorMuted = 3,
    ChatMuted = 4,
    MediaMuted = 5,
}

fn font(italic: bool) -> &'static Font {
    static ITALIC: OnceLock<Font> = OnceLock::new();
    static BOLD: OnceLock<Font> = OnceLock::new();
    let (cell, bytes) = if italic {
        (&ITALIC, TINOS_ITALIC)
    } else {
        (&BOLD, TINOS_BOLD)
    };
    cell.get_or_init(|| Font::from_bytes(bytes, fontdue::FontSettings::default()).expect("Tinos"))
}

pub struct ScreenKit {
    base: RgbImage,
    splash_img: RgbImage,
    tiles: Mutex<HashMap<(Face, u8, u8), Jpeg>>,
}

fn configured_kit() -> ScreenKit {
    ScreenKit {
        base: paint_screen(false),
        splash_img: paint_screen(true),
        tiles: Mutex::new(HashMap::new()),
    }
}

static KIT: OnceLock<ScreenKit> = OnceLock::new();

/// The GenoBeacon screen. Always available; BEACN artwork is not consulted.
pub fn kit() -> Option<&'static ScreenKit> {
    Some(KIT.get_or_init(configured_kit))
}

impl ScreenKit {
    fn region(&self, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        image::DynamicImage::ImageRgb8(imageops::crop_imm(&self.base, x, y, w, h).to_image())
            .to_rgba8()
    }

    /// Full background, sent once per redraw.
    pub fn background(&self) -> Result<Jpeg> {
        encode_rgb(&self.base, 92)
    }

    /// Top strip used by the non-card fallback layout. Same artwork as the header.
    pub fn header_strip(&self) -> Result<Jpeg> {
        let crop = imageops::crop_imm(&self.base, 0, 0, SCREEN.0, 80).to_image();
        encode_rgb(&crop, 90)
    }

    /// Boot screen, with an optional status line.
    pub fn splash(&self, status: Option<&str>) -> Result<Jpeg> {
        let mut img = image::DynamicImage::ImageRgb8(self.splash_img.clone()).to_rgba8();
        if let Some(text) = status {
            let (x, y) = STATUS_POS;
            draw_text(
                &mut img,
                text,
                x,
                y,
                STATUS_SIZE.0,
                STATUS_SIZE.1,
                false,
                20.0,
                DIM,
                true,
            );
        }
        encode_rgba(&img, 90)
    }

    pub fn status_strip(&self, text: &str) -> Result<Jpeg> {
        let (x, y) = STATUS_POS;
        let (w, h) = STATUS_SIZE;
        let mut img = image::DynamicImage::ImageRgb8(
            imageops::crop_imm(&self.splash_img, x, y, w, h).to_image(),
        )
        .to_rgba8();
        draw_text(&mut img, text, 0, 0, w, h, false, 20.0, DIM, true);
        encode_rgba(&img, 90)
    }

    /// One channel card: plate, name, colour line, and up to two routing lines.
    /// The dial JPEG is sent on top of the upper half.
    pub fn card(
        &self,
        slot: usize,
        title: &str,
        colour: [u8; 3],
        lines: &[String],
    ) -> Result<Jpeg> {
        let mut img = self.region(CARD_X[slot], CARD_Y, CARD_W, CARD_H);
        fill_round_rect(&mut img, 0, 0, CARD_W, CARD_H, 8, PANEL);
        stroke_round_rect(&mut img, 0, 0, CARD_W, CARD_H, 8, Rgba([42, 49, 66, 255]));
        fill_rect(&mut img, 0, BAND_Y, CARD_W, BAND_H, Rgba([12, 14, 20, 255]));
        for y in LINE_Y..LINE_Y + LINE_H {
            for x in 0..CARD_W {
                img.put_pixel(x, y, Rgba([colour[0], colour[1], colour[2], 255]));
            }
        }
        // A one-pixel red tick so the card reads as GenoBeacon even before the name.
        fill_rect(&mut img, 0, BAND_Y, 3, BAND_H, RED);
        draw_text(
            &mut img,
            title,
            8,
            BAND_Y,
            CARD_W - 16,
            BAND_H,
            true,
            LABEL_SIZE,
            PAPER,
            true,
        );
        for (line, centre) in lines.iter().take(2).zip(SUB_LINE_CENTRES) {
            draw_text(
                &mut img,
                line,
                8,
                centre - 11,
                CARD_W - 16,
                22,
                false,
                SUB_SIZE,
                DIM,
                true,
            );
        }
        encode_rgba(&img, 92)
    }

    pub fn blank_card(&self, slot: usize) -> Result<Jpeg> {
        let mut img = self.region(CARD_X[slot], CARD_Y, CARD_W, CARD_H);
        fill_round_rect(&mut img, 0, 0, CARD_W, CARD_H, 8, Rgba([10, 12, 16, 255]));
        stroke_round_rect(&mut img, 0, 0, CARD_W, CARD_H, 8, Rgba([28, 32, 42, 255]));
        encode_rgba(&img, 92)
    }

    pub fn button(&self, slot: usize, label: &str, active: bool) -> Result<Jpeg> {
        let mut img = self.region(button_x(slot), BUTTON_Y, BUTTON_W, BUTTON_H);
        let (fill, ink) = if active {
            (RED, PAPER)
        } else {
            (Rgba([18, 20, 28, 255]), DIM)
        };
        fill_round_rect(&mut img, 0, 0, BUTTON_W, BUTTON_H, 6, fill);
        if !active {
            stroke_round_rect(
                &mut img,
                0,
                0,
                BUTTON_W,
                BUTTON_H,
                6,
                Rgba([70, 76, 92, 255]),
            );
        }
        draw_text(
            &mut img,
            label,
            6,
            0,
            BUTTON_W - 12,
            BUTTON_H,
            false,
            BUTTON_SIZE,
            ink,
            true,
        );
        encode_rgba(&img, 92)
    }

    pub fn blank_button(&self, slot: usize) -> Result<Jpeg> {
        encode_rgba(
            &self.region(button_x(slot), BUTTON_Y, BUTTON_W, BUTTON_H),
            92,
        )
    }

    /// Personal (Mix A, blue) / audience (Mix B, red) toggle. `None` leaves the track empty.
    pub fn mix_pill(&self, mix: Option<Mix>) -> Result<Jpeg> {
        let (px, py) = PILL_POS;
        let (w, h) = PILL_SIZE;
        let mut img = self.region(px, py, w, h);
        fill_round_rect(&mut img, 0, 0, w, h, h / 2, Rgba([12, 14, 20, 255]));
        stroke_round_rect(&mut img, 0, 0, w, h, h / 2, Rgba([50, 56, 72, 255]));
        if let Some(mix) = mix {
            let knob = h.saturating_sub(8);
            let kx = match mix {
                Mix::A => 4,
                Mix::B => w - 4 - knob,
            };
            let colour = match mix {
                Mix::A => BLUE,
                Mix::B => RED,
            };
            fill_round_rect(&mut img, kx, 4, knob, knob, knob / 2, colour);
        }
        encode_rgba(&img, 92)
    }

    pub fn page_dots(&self, pages: u8, active: u8) -> Result<Jpeg> {
        let (dx, dy) = DOTS_POS;
        let (dw, dh) = DOTS_SIZE;
        let mut img = self.region(dx, dy, dw, dh);
        if pages > 1 {
            let n = pages.min(8) as u32;
            let gap = 16u32;
            let span = gap * n.saturating_sub(1);
            let first = dw / 2 - span / 2;
            for i in 0..n {
                let on = i as u8 == active;
                let colour = if on { BLUE } else { Rgba([70, 76, 92, 255]) };
                let r = if on { 4 } else { 3 };
                let cx = first + gap * i;
                fill_disc(&mut img, cx as i32, dh as i32 / 2, r, colour);
            }
        }
        encode_rgba(&img, 92)
    }

    /// Dial face for one (face, volume, meter) triple. Meter is already scaled to the volume.
    pub fn tile(&self, face: Face, volume: u8, meter: u8) -> Jpeg {
        let volume = volume.min(100);
        let meter = meter.min(volume) / 4 * 4;
        let key = (face, volume, meter);
        let mut cache = self.tiles.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(jpeg) = cache.get(&key) {
            return jpeg.clone();
        }
        if cache.len() > 512 {
            cache.clear();
        }
        let jpeg = encode_rgba(&draw_dial(face, volume, meter), 82)
            .unwrap_or_else(|_| Arc::new(vec![0xFF, 0xD8, 0xFF, 0xD9]));
        cache.insert(key, jpeg.clone());
        jpeg
    }

    /// 800×480 studio cue screen: the same chrome and dials, labeled for a BEACN Studio.
    /// Studio hardware has no LCD; this is the Studio visual in the device's pixel format,
    /// and the preview the tests write out.
    #[allow(dead_code)]
    pub fn studio_frame(&self) -> RgbImage {
        let mut img = image::DynamicImage::ImageRgb8(self.base.clone()).to_rgba8();
        draw_text(
            &mut img,
            "BEACN STUDIO",
            0,
            96,
            SCREEN.0,
            28,
            false,
            16.0,
            DIM,
            true,
        );
        let dials = [(120u32, "MIC", 72u8, 40u8), (512, "CUE", 48, 20)];
        for (x, label, volume, meter) in dials {
            let dial = draw_dial(Face::Volume, volume, meter);
            imageops::overlay(&mut img, &dial, x as i64, 150);
            draw_text(&mut img, label, x, 326, DIAL.0, 28, true, 18.0, PAPER, true);
        }
        draw_text(
            &mut img,
            "COMPLIANCE IS COMFORT",
            0,
            400,
            SCREEN.0,
            28,
            true,
            16.0,
            RED,
            true,
        );
        image::DynamicImage::ImageRgba8(img).to_rgb8()
    }
}

/// Which dial face a channel shows. `mute_a` / `mute_b` are (active, mutes to all).
pub fn face_for(is_source: bool, mute_a: (bool, bool), mute_b: (bool, bool), mix: Mix) -> Face {
    let (a_active, a_to_all) = mute_a;
    let (b_active, b_to_all) = mute_b;
    if !is_source {
        return if a_active { Face::Muted } else { Face::Volume };
    }
    if (a_active && (a_to_all || mix == Mix::A)) || (b_active && b_to_all) {
        Face::Muted
    } else if b_active && mix == Mix::B {
        Face::StreamMuted
    } else {
        Face::Volume
    }
}

fn paint_screen(splash: bool) -> RgbImage {
    let mut img = RgbaImage::from_pixel(SCREEN.0, SCREEN.1, BG);
    // Scanlines. Sparse on purpose: a terminal, not a texture copied from anywhere.
    for y in (0..SCREEN.1).step_by(4) {
        for x in 0..SCREEN.0 {
            let p = img.get_pixel_mut(x, y);
            p[0] = p[0].saturating_add(6);
            p[1] = p[1].saturating_add(7);
            p[2] = p[2].saturating_add(10);
        }
    }
    draw_mark(&mut img, 18, 16, 48);
    draw_text(
        &mut img,
        "GenoBeacon",
        76,
        14,
        420,
        52,
        true,
        32.0,
        PAPER,
        false,
    );
    draw_text(
        &mut img, "GENOCORP", 560, 18, 220, 26, false, 14.0, RED, false,
    );
    draw_text(
        &mut img,
        "COMPLIANCE IS COMFORT",
        430,
        44,
        350,
        20,
        true,
        12.0,
        DIM,
        false,
    );
    fill_rect(&mut img, 0, 78, SCREEN.0, 2, RED);
    fill_rect(&mut img, 0, 82, SCREEN.0, 1, BLUE);

    if splash {
        draw_mark(&mut img, SCREEN.0 / 2 - 36, 150, 72);
        draw_text(
            &mut img,
            "GenoBeacon",
            0,
            240,
            SCREEN.0,
            64,
            true,
            54.0,
            PAPER,
            true,
        );
        draw_text(
            &mut img,
            "A GENOCORP SOLUTIONS PRODUCT",
            0,
            310,
            SCREEN.0,
            28,
            false,
            14.0,
            DIM,
            true,
        );
        draw_text(
            &mut img,
            "FOR BEACN AUDIO DEVICES",
            0,
            338,
            SCREEN.0,
            24,
            false,
            13.0,
            BLUE,
            true,
        );
    }
    image::DynamicImage::ImageRgba8(img).to_rgb8()
}

/// Small radar mark: blue sweep, red beacon. Geometric, not a flame.
fn draw_mark(img: &mut RgbaImage, x: u32, y: u32, size: u32) {
    let cx = x as i32 + size as i32 / 2;
    let cy = y as i32 + size as i32 / 2;
    let r = size as f32 * 0.42;
    stroke_arc(
        img,
        cx,
        cy,
        r,
        (size as f32 * 0.08).max(2.0),
        200_f32.to_radians(),
        270_f32.to_radians(),
        BLUE,
    );
    let end = 200_f32.to_radians() + 270_f32.to_radians();
    let dx = cx as f32 + end.cos() * r;
    let dy = cy as f32 + end.sin() * r;
    fill_disc(
        img,
        dx.round() as i32,
        dy.round() as i32,
        (size / 10).max(2),
        RED,
    );
}

fn draw_dial(face: Face, volume: u8, meter: u8) -> RgbaImage {
    let (w, h) = DIAL;
    let mut img = RgbaImage::from_pixel(w, h, PANEL);
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;
    let radius = 62.0;
    let start = 135_f32.to_radians();
    let sweep = 270_f32.to_radians();
    stroke_arc(&mut img, cx, cy, radius, 8.0, start, sweep, TRACK);

    let vol = volume as f32 / 100.0;
    let met = meter as f32 / 100.0;
    let accent = match face {
        Face::Volume => BLUE,
        _ => RED,
    };
    if vol > 0.0 {
        stroke_arc(&mut img, cx, cy, radius, 8.0, start, sweep * vol, accent);
    }
    if met > 0.0 && matches!(face, Face::Volume | Face::StreamMuted) {
        stroke_arc(
            &mut img,
            cx,
            cy,
            radius - 12.0,
            3.0,
            start,
            sweep * met,
            METER,
        );
    }
    let (label, size, colour) = match face {
        Face::Volume => (volume.to_string(), 34.0, PAPER),
        Face::Muted => ("MUTE".to_string(), 20.0, RED),
        Face::StreamMuted => ("STREAM".to_string(), 15.0, RED),
        Face::MonitorMuted => ("MON".to_string(), 20.0, RED),
        Face::ChatMuted => ("CHAT".to_string(), 18.0, RED),
        Face::MediaMuted => ("MEDIA".to_string(), 16.0, RED),
    };
    draw_text(
        &mut img,
        &label,
        16,
        58,
        w - 32,
        52,
        false,
        size,
        colour,
        true,
    );
    img
}

#[allow(clippy::too_many_arguments)]
fn stroke_arc(
    img: &mut RgbaImage,
    cx: i32,
    cy: i32,
    radius: f32,
    thick: f32,
    start: f32,
    sweep: f32,
    colour: Rgba<u8>,
) {
    if sweep <= 0.0 {
        return;
    }
    let pad = (radius + thick + 2.0).ceil() as i32;
    let (width, height) = (img.width() as i32, img.height() as i32);
    for y in (cy - pad).max(0)..(cy + pad).min(height) {
        for x in (cx - pad).max(0)..(cx + pad).min(width) {
            let dx = x as f32 - cx as f32;
            let dy = y as f32 - cy as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let radial = ring_coverage(dist, radius, thick);
            if radial <= 0.0 {
                continue;
            }
            let mut ang = dy.atan2(dx);
            if ang < 0.0 {
                ang += TAU;
            }
            let mut from = start % TAU;
            if from < 0.0 {
                from += TAU;
            }
            let mut delta = ang - from;
            if delta < 0.0 {
                delta += TAU;
            }
            if delta <= sweep + 0.02 {
                blend(img, x, y, colour, radial);
            }
        }
    }
}

fn ring_coverage(dist: f32, radius: f32, thick: f32) -> f32 {
    let outer = radius + 0.8;
    let inner = radius - thick - 0.8;
    let outside = ((outer - dist) / 1.2).clamp(0.0, 1.0);
    let inside = ((dist - inner) / 1.2).clamp(0.0, 1.0);
    outside * inside
}

fn blend(img: &mut RgbaImage, x: i32, y: i32, colour: Rgba<u8>, coverage: f32) {
    if coverage <= 0.0 || x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    let a = coverage.clamp(0.0, 1.0);
    for i in 0..3 {
        p[i] = (colour[i] as f32 * a + p[i] as f32 * (1.0 - a)).round() as u8;
    }
    p[3] = 255;
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, colour: Rgba<u8>) {
    for py in y..y.saturating_add(h).min(img.height()) {
        for px in x..x.saturating_add(w).min(img.width()) {
            img.put_pixel(px, py, colour);
        }
    }
}

fn inside_round(px: i32, py: i32, x: i32, y: i32, w: i32, h: i32, r: i32) -> bool {
    let r = r.min(w / 2).min(h / 2).max(0);
    let (l, t, right, bottom) = (x, y, x + w, y + h);
    if px < l || py < t || px >= right || py >= bottom {
        return false;
    }
    let (cx, cy) = if px < l + r && py < t + r {
        (l + r, t + r)
    } else if px >= right - r && py < t + r {
        (right - r - 1, t + r)
    } else if px < l + r && py >= bottom - r {
        (l + r, bottom - r - 1)
    } else if px >= right - r && py >= bottom - r {
        (right - r - 1, bottom - r - 1)
    } else {
        return true;
    };
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy <= r * r
}

fn fill_round_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, r: u32, colour: Rgba<u8>) {
    let (xi, yi, wi, hi, ri) = (x as i32, y as i32, w as i32, h as i32, r as i32);
    for py in yi..yi + hi {
        for px in xi..xi + wi {
            if inside_round(px, py, xi, yi, wi, hi, ri) {
                blend(img, px, py, colour, 1.0);
            }
        }
    }
}

fn stroke_round_rect(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    r: u32,
    colour: Rgba<u8>,
) {
    let (xi, yi, wi, hi, ri) = (x as i32, y as i32, w as i32, h as i32, r as i32);
    for py in yi..yi + hi {
        for px in xi..xi + wi {
            let on = inside_round(px, py, xi, yi, wi, hi, ri)
                && !inside_round(px, py, xi + 1, yi + 1, wi - 2, hi - 2, ri.saturating_sub(1));
            if on {
                blend(img, px, py, colour, 1.0);
            }
        }
    }
}

fn fill_disc(img: &mut RgbaImage, cx: i32, cy: i32, r: u32, colour: Rgba<u8>) {
    let ri = r as i32;
    for y in cy - ri..=cy + ri {
        for x in cx - ri..=cx + ri {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= ri * ri {
                blend(img, x, y, colour, 1.0);
            }
        }
    }
}

pub fn text_width(text: &str, italic: bool, size: f32) -> f32 {
    let f = font(italic);
    text.chars().map(|c| f.metrics(c, size).advance_width).sum()
}

pub fn fit_text(text: &str, italic: bool, size: f32, max: f32) -> String {
    if text_width(text, italic, size) <= max {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = chars.iter().collect::<String>().trim_end().to_string() + "...";
        if text_width(&candidate, italic, size) <= max {
            return candidate;
        }
    }
    "...".into()
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    img: &mut RgbaImage,
    text: &str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    italic: bool,
    size: f32,
    colour: Rgba<u8>,
    center: bool,
) {
    let text = fit_text(text, italic, size, w as f32);
    let f = font(italic);
    let lm = f.horizontal_line_metrics(size).expect("line metrics");
    let total = lm.ascent - lm.descent;
    let baseline = y as f32 + (h as f32 - total) / 2.0 + lm.ascent;
    let width = text_width(&text, italic, size);
    let mut pen = if center {
        x as f32 + ((w as f32 - width) / 2.0).max(0.0)
    } else {
        x as f32
    };
    for c in text.chars() {
        let (m, bitmap) = f.rasterize(c, size);
        let gx = pen.round() as i32 + m.xmin;
        let gy = baseline.round() as i32 - m.ymin - m.height as i32;
        for row in 0..m.height {
            for col in 0..m.width {
                let a = bitmap[row * m.width + col] as f32 / 255.0;
                if a <= 0.0 {
                    continue;
                }
                blend(img, gx + col as i32, gy + row as i32, colour, a);
            }
        }
        pen += m.advance_width;
    }
}

fn encode_rgb(img: &RgbImage, quality: u8) -> Result<Jpeg> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut Cursor::new(&mut out), quality).encode(
        img.as_raw(),
        img.width(),
        img.height(),
        ExtendedColorType::Rgb8,
    )?;
    Ok(Arc::new(out))
}

fn encode_rgba(img: &RgbaImage, quality: u8) -> Result<Jpeg> {
    let rgb = RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        Rgb([p[0], p[1], p[2]])
    });
    encode_rgb(&rgb, quality)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    struct PreviewSlot<'a> {
        title: &'a str,
        colour: [u8; 3],
        lines: Vec<String>,
        volume: u8,
        meter: u8,
        face: Face,
        button: Option<(&'a str, bool)>,
    }

    fn paste(screen: &mut RgbImage, jpeg: &[u8], x: u32, y: u32) {
        let img = image::load_from_memory(jpeg).unwrap().to_rgb8();
        imageops::replace(screen, &img, x as i64, y as i64);
    }

    fn compose(
        kit: &ScreenKit,
        slots: &[Option<PreviewSlot>],
        mix: Option<Mix>,
        pages: u8,
        page: u8,
        create: bool,
    ) -> RgbImage {
        let mut s = RgbImage::new(SCREEN.0, SCREEN.1);
        paste(&mut s, &kit.background().unwrap(), 0, 0);
        paste(&mut s, &kit.mix_pill(mix).unwrap(), PILL_POS.0, PILL_POS.1);
        paste(
            &mut s,
            &kit.page_dots(pages, page).unwrap(),
            DOTS_POS.0,
            DOTS_POS.1,
        );
        let (dx, dy) = dial_offset();
        for (i, slot) in slots.iter().enumerate() {
            match slot {
                None => {
                    paste(&mut s, &kit.blank_card(i).unwrap(), CARD_X[i], CARD_Y);
                    if create {
                        paste(&mut s, &kit.blank_button(i).unwrap(), button_x(i), BUTTON_Y);
                    }
                }
                Some(c) => {
                    paste(
                        &mut s,
                        &kit.card(i, c.title, c.colour, &c.lines).unwrap(),
                        CARD_X[i],
                        CARD_Y,
                    );
                    let tile = kit.tile(c.face, c.volume, c.meter);
                    paste(
                        &mut s,
                        &tile,
                        (CARD_X[i] as i32 + dx) as u32,
                        (CARD_Y as i32 + dy) as u32,
                    );
                    if create {
                        match c.button {
                            Some((label, on)) => {
                                paste(
                                    &mut s,
                                    &kit.button(i, label, on).unwrap(),
                                    button_x(i),
                                    BUTTON_Y,
                                );
                            }
                            None => {
                                paste(&mut s, &kit.blank_button(i).unwrap(), button_x(i), BUTTON_Y)
                            }
                        }
                    }
                }
            }
        }
        s
    }

    #[test]
    fn face_rules() {
        assert_eq!(
            face_for(true, (true, true), (false, false), Mix::B),
            Face::Muted
        );
        assert_eq!(
            face_for(true, (true, false), (false, false), Mix::A),
            Face::Muted
        );
        assert_eq!(
            face_for(true, (true, false), (false, false), Mix::B),
            Face::Volume
        );
        assert_eq!(
            face_for(true, (false, false), (true, false), Mix::B),
            Face::StreamMuted
        );
        assert_eq!(
            face_for(true, (false, false), (true, false), Mix::A),
            Face::Volume
        );
        assert_eq!(
            face_for(false, (true, true), (false, false), Mix::A),
            Face::Muted
        );
        assert_eq!(
            face_for(true, (false, false), (true, true), Mix::A),
            Face::Muted
        );
        assert_eq!(
            face_for(true, (false, false), (true, true), Mix::B),
            Face::Muted
        );
    }

    #[test]
    fn ellipsis_fits() {
        let long = "Microphone Array (Realtek High Definition Audio)";
        let t = fit_text(long, false, SUB_SIZE, 175.0);
        assert!(t.ends_with("..."));
        assert!(text_width(&t, false, SUB_SIZE) <= 175.0);
        assert_eq!(fit_text("Mic", true, LABEL_SIZE, 175.0), "Mic");
    }

    #[test]
    fn every_face_encodes() {
        let kit = kit().unwrap();
        for face in [
            Face::Volume,
            Face::Muted,
            Face::StreamMuted,
            Face::MonitorMuted,
            Face::ChatMuted,
            Face::MediaMuted,
        ] {
            let jpeg = kit.tile(face, 40, 16);
            assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
        }
    }

    #[test]
    fn frames_are_device_jpegs() {
        let kit = kit().unwrap();
        for jpeg in [
            kit.background().unwrap(),
            kit.splash(Some("LINKED")).unwrap(),
        ] {
            assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
            let img = image::load_from_memory(&jpeg).unwrap();
            assert_eq!(img.dimensions(), SCREEN);
        }
        let dial = kit.tile(Face::Volume, 40, 20);
        let img = image::load_from_memory(&dial).unwrap();
        assert_eq!(img.dimensions(), DIAL);
        assert_eq!(kit.studio_frame().dimensions(), SCREEN);
    }

    #[test]
    #[ignore]
    fn render_previews() {
        let out = std::path::PathBuf::from(
            std::env::var("GENOBEACON_PREVIEW_DIR")
                .unwrap_or_else(|_| "/tmp/genobeacon-preview".into()),
        );
        std::fs::create_dir_all(&out).unwrap();
        let kit = kit().unwrap();
        let s = |v: Vec<&str>| v.into_iter().map(String::from).collect::<Vec<_>>();
        let sources = |mix: Mix| {
            vec![
                Some(PreviewSlot {
                    title: "System",
                    colour: [61, 126, 255],
                    lines: s(vec!["Firefox", "Steam"]),
                    volume: 76,
                    meter: 40,
                    face: Face::Volume,
                    button: Some(("AUDIENCE", false)),
                }),
                Some(PreviewSlot {
                    title: "Browser",
                    colour: [225, 6, 0],
                    lines: s(vec!["Brave"]),
                    volume: 63,
                    meter: 0,
                    face: face_for(true, (true, true), (false, false), mix),
                    button: Some(("AUDIENCE", false)),
                }),
                Some(PreviewSlot {
                    title: "Music",
                    colour: [120, 150, 255],
                    lines: s(vec!["Spotify"]),
                    volume: if mix == Mix::A { 41 } else { 20 },
                    meter: 30,
                    face: face_for(true, (false, false), (true, false), mix),
                    button: Some(("AUDIENCE", true)),
                }),
                Some(PreviewSlot {
                    title: "Microphone",
                    colour: [232, 234, 240],
                    lines: s(vec!["BEACN Studio"]),
                    volume: 100,
                    meter: 48,
                    face: Face::Volume,
                    button: Some(("MUTE ALL", false)),
                }),
            ]
        };
        compose(kit, &sources(Mix::A), Some(Mix::A), 2, 0, true)
            .save(out.join("mix-create-mix-a.png"))
            .unwrap();
        compose(kit, &sources(Mix::B), Some(Mix::B), 2, 0, true)
            .save(out.join("mix-create-mix-b.png"))
            .unwrap();
        image::load_from_memory(&kit.splash(Some("LINKED")).unwrap())
            .unwrap()
            .save(out.join("mix-create-splash.png"))
            .unwrap();
        kit.studio_frame().save(out.join("studio.png")).unwrap();
        println!("previews in {}", out.display());
    }
}
