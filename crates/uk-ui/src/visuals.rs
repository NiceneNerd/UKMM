use std::sync::LazyLock;

use color_hex::color_from_hex;
use egui::{
    epaint::{Margin, RectShape, Shadow, Tessellator},
    style::{Selection, Spacing, TextCursorStyle, WidgetVisuals, Widgets},
    vec2, Color32, FontFamily, LayerId, Mesh, Rect, Rounding, Stroke, Style, Ui, Visuals,
};
use egui_aesthetix::Aesthetix;
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
    let mut dock_style = egui_dock::Style::from_egui(style);
    dock_style.tab.tab_body.corner_radius = Rounding {
        ne: 2,
        nw: 2,
        ..Default::default()
    };
    dock_style.tab.focused.text_color = style.visuals.strong_text_color();
    dock_style.tab.inactive.text_color = style.visuals.weak_text_color();
    dock_style.tab.tab_body.stroke.color = style.visuals.widgets.noninteractive.bg_stroke.color;
    dock_style.tab.active.outline_color = style.visuals.widgets.noninteractive.bg_stroke.color;
    dock_style.separator.width = 0.5;
    dock_style.separator.color_idle = style.visuals.widgets.noninteractive.bg_stroke.color;
    dock_style.separator.color_dragged = style.visuals.widgets.active.bg_stroke.color;
    dock_style.separator.color_hovered = style.visuals.widgets.active.bg_stroke.color;
    dock_style.dock_area_padding = Some(Margin::default());
    dock_style
}

pub fn slate_grid(ui: &mut Ui) {
    ui.with_layer_id(LayerId::background(), |ui| {
        let cursor = ui.cursor();
        let width = ui.available_width();
        let height = ui.available_height() * 1.5;
        static GRID_COLOR: LazyLock<Color32> = LazyLock::new(|| BLUE.linear_multiply(0.0333));
        const GRID_OFFSET: f32 = 16.0;
        let bg_rect = Rect::from_min_size(ui.cursor().min, ui.available_size()); //.shrink(4.0);
        ui.painter()
            .rect_filled(bg_rect, Rounding::ZERO, ui.style().visuals.extreme_bg_color);
        ui.set_clip_rect(bg_rect);
        ui.painter().add({
            let mut mesh = Mesh::default();
            let mut tesselator = Tessellator::new(
                ui.fonts(|f| f.pixels_per_point()),
                egui::epaint::TessellationOptions {
                    feathering: true,
                    feathering_size_in_pixels: 32.0,
                    ..Default::default()
                },
                [0, 0],
                vec![],
            );
            tesselator.tessellate_rect(
                &RectShape::filled(
                    bg_rect.expand2([64.0, 0.0].into()),
                    0.0,
                    ui.style().visuals.widgets.inactive.bg_fill,
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
    AdwaitaDark,
    AdwaitaLight,
    Carl,
    SweetDark,
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
            Theme::AdwaitaDark => "Adwaita Dark",
            Theme::AdwaitaLight => "Adwaita Light",
            Theme::Carl => "Carl",
            Theme::SweetDark => "Sweet Dark",
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
            Theme::AdwaitaDark,
            Theme::AdwaitaLight,
            Theme::Carl,
            Theme::SweetDark,
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
                                bg_fill: hex_color!("#1C1E1F"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#2F2E2A")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#BCCAD1")),
                                corner_radius: Rounding::same(0),
                                expansion: 0.0,
                                weak_bg_fill: Color32::TRANSPARENT,
                            },
                            inactive: WidgetVisuals {
                                bg_fill: hex_color!("#1d4e77"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#237ba3")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#f0f0f0")),
                                corner_radius: Rounding::same(2),
                                expansion: 0.0,
                                weak_bg_fill: Color32::TRANSPARENT,
                            },
                            hovered: WidgetVisuals {
                                bg_fill: hex_color!("#237ba3"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#1d649a")),
                                fg_stroke: Stroke::new(1.5, hex_color!("#f0f0f0")),
                                corner_radius: Rounding::same(2),
                                expansion: 1.0,
                                weak_bg_fill: Color32::TRANSPARENT,
                            },
                            active: WidgetVisuals {
                                bg_fill: hex_color!("#12384f"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#237ba3")),
                                fg_stroke: Stroke::new(1.5, hex_color!("#D9EEFF")),
                                corner_radius: Rounding::same(2),
                                expansion: 1.0,
                                weak_bg_fill: Color32::TRANSPARENT,
                            },
                            open: WidgetVisuals {
                                bg_fill: hex_color!("#1C1E1F"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#2F2E2A")),
                                fg_stroke: Stroke::new(1.0, hex_color!("#D9EEFF")),
                                corner_radius: Rounding::same(2),
                                expansion: 0.0,
                                weak_bg_fill: Color32::TRANSPARENT,
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
                        window_corner_radius: Rounding::same(4),
                        window_shadow: Shadow {
                            offset: [0, 0],
                            blur: 5,
                            spread: 5,
                            color: Color32::from_black_alpha(45),
                        },
                        popup_shadow: Shadow {
                            offset: [0, 0],
                            blur: 5,
                            spread: 5,
                            color: Color32::from_black_alpha(45),
                        },
                        window_fill: hex_color!("#1C1E1F"),
                        window_stroke: Stroke::NONE,
                        panel_fill: hex_color!("#1C1E1F"),
                        resize_corner_size: 8.0,
                        text_cursor: TextCursorStyle {
                            preview: false,
                            ..Default::default()
                        },
                        clip_rect_margin: 3.0, /* should be at least half the size of the widest
                                                * frame stroke
                                                * + max WidgetVisuals::expansion */
                        button_frame: true,
                        collapsing_header_frame: false,
                        ..Default::default()
                    },
                    spacing: Spacing {
                        button_padding: [4.0, 2.0].into(),
                        icon_spacing: 4.0,
                        menu_margin: Margin::same(4),
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
            Self::AdwaitaDark => {
                // TODO: Update egui_aesthetix for egui 0.32 compatibility
                ctx.set_visuals(egui::style::Visuals::dark());
            }
            Self::AdwaitaLight => {
                // TODO: Update egui_aesthetix for egui 0.32 compatibility  
                ctx.set_visuals(egui::style::Visuals::light());
            }
            Self::Carl => {
                // TODO: Update egui_aesthetix for egui 0.32 compatibility
                ctx.set_visuals(egui::style::Visuals::dark());
            }
            Self::SweetDark => {
                ctx.set_style(Style {
                    visuals: Visuals {
                        dark_mode: true,
                        override_text_color: None,
                        widgets: Widgets {
                            noninteractive: WidgetVisuals {
                                weak_bg_fill: hex_color!("#181B28"),
                                bg_fill: hex_color!("#181B28"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#2F3B51")), // separators, indentation lines
                                fg_stroke: Stroke::new(1.0, hex_color!("#EEEEEE")), // normal text color
                                corner_radius: Rounding::same(2),
                                expansion: 0.0,
                            },
                            inactive: WidgetVisuals {
                                weak_bg_fill: hex_color!("#1B1E2D"), // button background
                                bg_fill: hex_color!("#303651"),      // checkbox background
                                bg_stroke: Stroke {
                                    color: hex_color!("#12141e"),
                                    width: 1.0,
                                },
                                fg_stroke: Stroke::new(1.0, hex_color!("#fefefe")), // button text
                                corner_radius: Rounding::same(2),
                                expansion: 0.0,
                            },
                            hovered: WidgetVisuals {
                                weak_bg_fill: hex_color!("#262C45"),
                                bg_fill: hex_color!("#262C45"),
                                bg_stroke: Stroke::new(1.0, hex_color!("#71f79f")), // e.g. hover over window edge or button
                                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                                corner_radius: Rounding::same(3),
                                expansion: 0.5,
                            },
                            active: WidgetVisuals {
                                weak_bg_fill: hex_color!("#31363D"),
                                bg_fill: hex_color!("#31363D"),
                                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                                corner_radius: Rounding::same(2),
                                expansion: 0.5,
                            },
                            open: WidgetVisuals {
                                weak_bg_fill: hex_color!("#262C45"),
                                bg_fill: hex_color!("#c74ded"),
                                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                                fg_stroke: Stroke::new(1.0, Color32::from_gray(210)),
                                corner_radius: Rounding::same(2),
                                expansion: 0.0,
                            },
                        },
                        selection: Selection {
                            bg_fill: hex_color!("#c74ded"),
                            stroke: Stroke {
                                width: 1.0,
                                color: Color32::from_gray(230),
                            },
                        },
                        hyperlink_color: hex_color!("#c74ded"),
                        faint_bg_color: hex_color!("#161925"), // visible, but barely so
                        extreme_bg_color: hex_color!("#181B21"), // e.g. TextEdit background
                        code_bg_color: hex_color!("#0C0E15"),
                        warn_fg_color: hex_color!("#ff6a00"), // orange
                        error_fg_color: hex_color!("#ed254e"), // red
                        window_corner_radius: Rounding::same(4),
                        window_shadow: Shadow {
                            offset: [10, 20],
                            blur: 15,
                            spread: 0,
                            color: Color32::from_black_alpha(96),
                        },
                        window_fill: hex_color!("#181B28"),
                        window_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                        window_highlight_topmost: true,
                        panel_fill: hex_color!("#181B28"),
                        popup_shadow: Shadow {
                            offset: [6, 10],
                            blur: 8,
                            spread: 0,
                            color: Color32::from_black_alpha(96),
                        },
                        resize_corner_size: 12.0,
                        text_cursor: TextCursorStyle {
                            preview: false,
                            stroke: Stroke::new(2.0, Color32::from_rgb(192, 222, 255)),
                            ..Default::default()
                        },
                        clip_rect_margin: 3.0, // should be at least half the size of the widest frame stroke + max WidgetVisuals::expansion
                        button_frame: true,
                        collapsing_header_frame: false,
                        indent_has_left_vline: true,
                        striped: false,
                        slider_trailing_fill: false,
                        handle_shape: egui::style::HandleShape::Circle,
                        interact_cursor: None,
                        image_loading_spinners: true,
                        numeric_color_space: egui::style::NumericColorSpace::GammaByte,
                        ..Default::default() // Handle any other new fields
                    },
                    spacing: Spacing {
                        item_spacing: vec2(8.0, 4.0),
                        window_margin: Margin::same(8),
                        menu_margin: Margin::same(8),
                        button_padding: vec2(8.0, 4.0),
                        indent: 28.0, // match checkbox/radio-button with `button_padding.x + icon_width + icon_spacing`
                        interact_size: vec2(48.0, 20.0),
                        slider_width: 100.0,
                        slider_rail_height: 8.0,
                        combo_width: 100.0,
                        text_edit_width: 280.0,
                        icon_width: 16.0,
                        icon_width_inner: 10.0,
                        icon_spacing: 6.0,
                        tooltip_width: 600.0,
                        menu_width: 160.0,
                        menu_spacing: 4.0,
                        combo_height: 200.0,
                        scroll: Default::default(),
                        indent_ends_with_horizontal_line: false,
                        default_area_size: vec2(600.0, 400.0)
                    },
                    ..Default::default()
                });
            }
        }
    }
}
