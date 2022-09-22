use eframe::epaint::color_hex::color_from_hex;
use egui::{
    hex_color,
    style::{WidgetVisuals, Widgets},
    Color32, Rounding, Stroke, Visuals,
};

macro_rules! from_hex {
    ($hex:expr) => {{
        let _arr = color_from_hex!($hex);
        Color32::from_rgb(_arr[0], _arr[1], _arr[2])
    }};
}

pub const GREEN: Color32 = from_hex!("#528f24");
pub const BLUE: Color32 = from_hex!("#38b6f1");
pub const RED: Color32 = from_hex!("#F52331");
pub const YELLOW: Color32 = from_hex!("#ffbc28");
pub const ORGANGE: Color32 = from_hex!("#ff953f");

const fn darken(color: Color32, percentage: usize) -> Color32 {
    Color32::from_rgb(
        (((color.r() as usize * 100) - (percentage * 100)) / 100) as u8,
        (((color.g() as usize * 100) - (percentage * 100)) / 100) as u8,
        (((color.b() as usize * 100) - (percentage * 100)) / 100) as u8,
    )
}

const fn lighten(color: Color32, percentage: usize) -> Color32 {
    Color32::from_rgb(
        (((color.r() as usize * 100) + (percentage * 100)) / 100) as u8,
        (((color.g() as usize * 100) + (percentage * 100)) / 100) as u8,
        (((color.b() as usize * 100) + (percentage * 100)) / 100) as u8,
    )
}

#[inline(always)]
pub fn panel() -> Color32 {
    default().widgets.noninteractive.bg_fill
}

#[inline(always)]
pub fn dark_panel() -> Color32 {
    darken(panel(), 25)
}

pub fn default() -> Visuals {
    Visuals {
        widgets: Widgets {
            noninteractive: WidgetVisuals {
                bg_fill: hex_color!("#302e31"),
                rounding: Rounding::same(4.),
                bg_stroke: Stroke::new(1., lighten(hex_color!("#302e31"), 10)),
                expansion: 0.,
                fg_stroke: Stroke::new(2., hex_color!("#e0e0e0")),
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
