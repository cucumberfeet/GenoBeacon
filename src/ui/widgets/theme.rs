use iced::theme::Palette;
use iced::theme::palette::Extended;
use iced::{Color, Theme};

pub fn build_genobeacon_theme() -> Theme {
    let base_palette = Palette {
        background: Color::from_rgb8(0x07, 0x08, 0x0C),
        text: Color::from_rgb8(0xC8, 0xCC, 0xD6),
        primary: Color::from_rgb8(0x3D, 0x7E, 0xFF),
        success: Color::from_rgb8(40, 167, 69),
        warning: Color::from_rgb8(255, 193, 7),
        danger: Color::from_rgb8(0xE1, 0x06, 0x00),
    };

    Theme::custom_with_fn("GenoBeacon".to_string(), base_palette, |palette| {
        // Generate the default extended palette template from the base colors
        let mut extended = Extended::generate(palette);

        // Override the exact background subfield that naked rules pull from by default
        extended.background.strong.color = Color::from_rgb8(0x16, 0x1A, 0x22);
        extended.background.base.text = palette.text;

        extended
    })
}
