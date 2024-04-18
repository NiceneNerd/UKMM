use egui_aesthetix::Aesthetix;
use super::color_from_hex;

pub struct SweetDark;

impl Aesthetix for SweetDark {
    fn name(&self) -> &str {
        "Sweet Dark"
    }

    fn primary_accent_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(151, 0, 190) // textLink.foreground
    }

    fn secondary_accent_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(234, 220, 53) // extensionButton.prominentForeground
    }

    fn bg_primary_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(34, 34, 53) // editor.background
    }

    fn bg_secondary_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(27, 27, 42) // sideBar.background
    }

    fn bg_triage_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(27, 27, 42) // sideBarSectionHeader.background
    }

    fn bg_auxiliary_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(30, 30, 53) // dropdown.background
    }

    fn bg_contrast_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(34, 34, 53) // editorGroup.background
    }

    fn fg_primary_text_color_visuals(&self) -> Option<egui::Color32> {
        Some(egui::Color32::from_rgb(255, 255, 255)) // foreground
    }

    fn fg_success_text_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(0, 255, 0) // merge.incomingContentBackground
    }

    fn fg_warn_text_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(246, 145, 84) // editorWarning.foreground
    }

    fn fg_error_text_color_visuals(&self) -> egui::Color32 {
        egui::Color32::from_rgb(246, 0, 85) // errorForeground
    }

    fn dark_mode_visuals(&self) -> bool {
        true
    }

    fn margin_style(&self) -> f32 {
        8.0
    }

    fn button_padding(&self) -> egui::Vec2 {
        egui::Vec2 { x: 8.0, y: 6.0 }
    }

    fn item_spacing_style(&self) -> f32 {
        12.0
    }

    fn scroll_bar_width_style(&self) -> f32 {
        10.0
    }

    fn rounding_visuals(&self) -> f32 {
        6.0
    }
}