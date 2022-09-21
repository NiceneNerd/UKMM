use egui::{
    hex_color,
    style::{WidgetVisuals, Widgets},
    Color32, Rounding, Stroke, Visuals,
};

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
