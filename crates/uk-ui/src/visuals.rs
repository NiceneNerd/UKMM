use color_hex::color_from_hex;
use egui::{
    epaint::{RectShape, Shadow, Tessellator},
    style::{Margin, Selection, Spacing, WidgetVisuals, Widgets},
    Color32, FontFamily, LayerId, Mesh, Rect, Rounding, Stroke, Style, Ui, Visuals,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

macro_rules! hex_color {
    ($hex:expr) => {{
        let _arr = color_from_hex!($hex);
        Color32::from_rgb(_arr[0], _arr[1], _arr[2])
    }};
}

pub const GREEN: Color32 = hex_color!("#528f24");
pub const BLUE: Color32 = hex_color!("#38b6f1");
pub const RED: Color32 = hex_color!("#F52331");
pub const YELLOW: Color32 = hex_color!("#ffbc28");
pub const ORGANGE: Color32 = hex_color!("#ff953f");

#[inline(always)]
pub fn error_bg(visuals: &Visuals) -> Color32 {
    let mut color = egui::ecolor::Hsva::from(RED);
    color.v = egui::ecolor::Hsva::from(visuals.window_fill()).v;
    color.into()
}

pub fn style_dock(style: &egui::Style) -> egui_dock::Style {
    egui_dock::StyleBuilder::from_egui(style)
        .show_close_buttons(false)
        .with_tab_rounding(Rounding {
            ne: 2.0,
            nw: 2.0,
            ..Default::default()
        })
        .with_tab_text_color_focused(style.visuals.strong_text_color())
        .with_tab_text_color_unfocused(style.visuals.weak_text_color())
        .with_tab_outline_color(style.visuals.widgets.noninteractive.bg_stroke.color)
        .with_border_width(1.0)
        .with_border_color(style.visuals.widgets.noninteractive.bg_stroke.color)
        .with_separator_width(1.0)
        .with_separator_color(style.visuals.widgets.noninteractive.bg_stroke.color)
        .with_padding(Margin::default())
        .build()
}

pub fn slate_grid(ui: &mut Ui) {
    ui.with_layer_id(LayerId::background(), |ui| {
        let cursor = ui.cursor();
        let width = ui.available_width();
        let height = ui.available_height() * 1.5;
        static GRID_COLOR: Lazy<Color32> = Lazy::new(|| BLUE.linear_multiply(0.0333));
        const GRID_OFFSET: f32 = 16.0;
        let bg_rect = Rect::from_min_size(ui.cursor().min, ui.available_size()); //.shrink(4.0);
        ui.painter().rect_filled(
            bg_rect,
            Rounding::none(),
            ui.style().visuals.extreme_bg_color,
        );
        ui.set_clip_rect(bg_rect);
        ui.painter().add({
            let mut mesh = Mesh::default();
            let mut tesselator = Tessellator::new(
                ui.fonts().pixels_per_point(),
                egui::epaint::TessellationOptions {
                    feathering: true,
                    feathering_size_in_pixels: 32.0,
                    ..Default::default()
                },
                [0, 0],
                vec![],
            );
            tesselator.tessellate_rect(
                &RectShape::stroke(
                    bg_rect.expand2([64.0, 0.0].into()),
                    0.0,
                    Stroke::new(2.0, ui.style().visuals.widgets.inactive.bg_fill),
                ),
                &mut mesh,
            );
            mesh
        });
        for i in 0..(height as usize / 48 + 1) {
            ui.painter().hline(
                cursor.min.x..=width + 4.0,
                (i as f32 * 48.0) + cursor.min.y + GRID_OFFSET,
                Stroke::new(1.0, *GRID_COLOR),
            );
        }
        for i in 0..(width as usize / 48 + 1) {
            ui.painter().vline(
                (i as f32 * 48.0) + cursor.min.x + GRID_OFFSET,
                cursor.min.y..=height,
                Stroke::new(1.0, *GRID_COLOR),
            );
        }
    });
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Theme {
    #[default]
    Sheikah,
    Egui,
    EguiLight,
    Frappe,
    Latte,
    Macchiato,
    Mocha,
}

impl Theme {
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Theme::Sheikah => "Sheikah Slate",
            Theme::Egui => "egui Dark",
            Theme::EguiLight => "egui Light",
            Theme::Frappe => "Frappe",
            Theme::Latte => "Latte",
            Theme::Macchiato => "Macchiato",
            Theme::Mocha => "Mocha",
        }
    }

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Theme::Sheikah,
            Theme::Egui,
            Theme::EguiLight,
            Theme::Frappe,
            Theme::Latte,
            Theme::Macchiato,
            Theme::Mocha,
        ]
        .into_iter()
    }

    pub fn set_theme(&self, ctx: &egui::Context) {
        match self {
            Self::Sheikah => {
                ctx.set_style(Style {
                    animation_time: 0.2,
                    visuals: Visuals {
                        dark_mode: true,
                        override_text_color: None,
                        widgets: Widgets {
                            noninteractive: WidgetVisuals {
                                bg_fill:   hex_color!("#1C1E1F"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#2F2E2A")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#BCCAD1")),
                                rounding:  Rounding::same(0.0),
                                expansion: 0.0,
                            },
                            inactive: WidgetVisuals {
                                bg_fill:   hex_color!("#1d4e77"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#237ba3")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#f0f0f0")),
                                rounding:  Rounding::same(2.0),
                                expansion: 0.0,
                            },
                            hovered: WidgetVisuals {
                                bg_fill:   hex_color!("#237ba3"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#1d649a")),
                                fg_stroke: Stroke::new(1.5, hex_color!("#f0f0f0")),
                                rounding:  Rounding::same(2.0),
                                expansion: 1.0,
                            },
                            active: WidgetVisuals {
                                bg_fill:   hex_color!("#12384f"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#237ba3")),
                                fg_stroke: Stroke::new(1.5, hex_color!("#D9EEFF")),
                                rounding:  Rounding::same(2.0),
                                expansion: 1.0,
                            },
                            open: WidgetVisuals {
                                bg_fill:   hex_color!("#1C1E1F"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#2F2E2A")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#D9EEFF")),
                                rounding:  Rounding::same(2.0),
                                expansion: 0.0,
                            },
                        },
                        selection: Selection {
                            bg_fill: BLUE.linear_multiply(0.667),
                            stroke:  Stroke::new(1.0, Color32::WHITE),
                        },
                        hyperlink_color: BLUE,
                        faint_bg_color: hex_color!("#252729"),
                        extreme_bg_color: hex_color!("#030a0e"), // e.g. TextEdit background
                        code_bg_color: Color32::from_gray(32),
                        warn_fg_color: ORGANGE, // orange
                        error_fg_color: RED,    // red
                        window_rounding: Rounding::same(4.0),
                        window_shadow: Shadow::big_dark(),
                        popup_shadow: Shadow::small_dark(),
                        window_fill: hex_color!("#1C1E1F"),
                        window_stroke: Stroke::NONE,
                        panel_fill: hex_color!("#1C1E1F"),
                        resize_corner_size: 8.0,
                        text_cursor_width: 2.0,
                        text_cursor_preview: false,
                        clip_rect_margin: 3.0, /* should be at least half the size of the widest
                                                * frame stroke
                                                * + max WidgetVisuals::expansion */
                        button_frame: true,
                        collapsing_header_frame: false,
                    },
                    spacing: Spacing {
                        button_padding: [4.0, 2.0].into(),
                        icon_spacing: 4.0,
                        menu_margin: Margin::same(4.0),
                        scroll_bar_width: 2.0,
                        indent_ends_with_horizontal_line: false,
                        ..Default::default()
                    },
                    text_styles: {
                        let mut styles = egui::style::default_text_styles();
                        styles.get_mut(&egui::TextStyle::Heading).unwrap().family =
                            FontFamily::Name("Bold".into());
                        styles
                    },
                    ..Default::default()
                });
            }
            Self::Egui => {
                ctx.set_visuals(egui::style::Visuals::dark());
            }
            Self::EguiLight => {
                ctx.set_visuals(egui::style::Visuals::light());
            }
            Self::Frappe => {
                catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE);
            }
            Self::Latte => {
                catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE);
            }
            Self::Macchiato => {
                catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO);
            }
            Self::Mocha => {
                catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
            }
        }
    }
}
