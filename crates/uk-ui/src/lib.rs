pub mod ext;
pub mod icons;
mod paths;
pub mod syntect;
pub mod visuals;
pub use egui;
pub use egui_dock;
pub use egui_extras;
use font_loader::system_fonts::FontPropertyBuilder;
use include_flate::flate;
pub use paths::PathNode;

flate!(static NOTOSANS_REG: [u8] from "../../fonts/NotoSans-Regular.ttf");
flate!(static NOTOSANS_BOLD: [u8] from "../../fonts/NotoSans-Bold.ttf");
flate!(static NOTOSANSJP_REG: [u8] from "../../fonts/NotoSansJP-Regular.ttf");
flate!(static NOTOSANSJP_BOLD: [u8] from "../../fonts/NotoSansJP-Bold.ttf");
flate!(static NOTOSANSKR_REG: [u8] from "../../fonts/NotoSansKR-Regular.ttf");
flate!(static NOTOSANSKR_BOLD: [u8] from "../../fonts/NotoSansKR-Bold.ttf");
flate!(static NOTOSANSSC_REG: [u8] from "../../fonts/NotoSansSC-Regular.ttf");
flate!(static NOTOSANSSC_BOLD: [u8] from "../../fonts/NotoSansSC-Bold.ttf");

pub fn load_fonts(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_to_try = if cfg!(windows) {
        "Segoe UI".to_owned()
    } else if cfg!(target_os = "macos") {
        "Lucida Grande".to_owned()
    } else {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "font-name"])
            .output()
            .ok()
            .and_then(|o| (!o.stdout.is_empty()).then_some(o))
            .and_then(|o| {
                String::from_utf8(o.stdout)
                    .map(|s| {
                        let last_space = s.rfind(' ').unwrap();
                        s[..last_space].trim_matches('\'').to_string()
                    })
                    .ok()
            })
            .unwrap_or_else(|| "Ubuntu".to_owned())
    };
    if let Some(system_font) =
        font_loader::system_fonts::get(&FontPropertyBuilder::new().family(&font_to_try).build())
    {
        fonts.font_data.insert(
            "System".to_owned(),
            egui::FontData::from_owned(system_font.0).into(),
        );
    }
    if let Some(system_font) = font_loader::system_fonts::get(
        &FontPropertyBuilder::new()
            .family(&font_to_try)
            .bold()
            .build(),
    )
    .or_else(|| {
        let property = FontPropertyBuilder::new()
            .family(&(font_to_try + " Bold"))
            .build();
        font_loader::system_fonts::get(&property)
    }) {
        fonts
            .font_data
            .insert("Bold".to_owned(), egui::FontData::from_owned(system_font.0).into());
    }
    fonts.font_data.insert(
        "Noto".to_owned(),
        egui::FontData::from_static(&NOTOSANS_REG).into()
    );
    fonts.font_data.insert(
        "NotoBold".to_owned(),
        egui::FontData::from_static(&NOTOSANS_BOLD).into()
    );
    fonts.font_data.insert(
        "NotoJP".to_owned(),
        egui::FontData::from_static(&NOTOSANSJP_REG).into()
    );
    fonts.font_data.insert(
        "NotoJPBold".to_owned(),
        egui::FontData::from_static(&NOTOSANSJP_BOLD).into()
    );
    fonts.font_data.insert(
        "NotoKR".to_owned(),
        egui::FontData::from_static(&NOTOSANSKR_REG).into()
    );
    fonts.font_data.insert(
        "NotoKRBold".to_owned(),
        egui::FontData::from_static(&NOTOSANSKR_BOLD).into()
    );
    fonts.font_data.insert(
        "NotoSC".to_owned(),
        egui::FontData::from_static(&NOTOSANSSC_REG).into()
    );
    fonts.font_data.insert(
        "NotoSCBold".to_owned(),
        egui::FontData::from_static(&NOTOSANSSC_BOLD).into()
    );
    if let Some(family) = fonts
        .families
        .get_mut(&egui::FontFamily::Proportional) {
        ["NotoSC", "NotoKR", "NotoJP", "Noto", "System"].iter().for_each(|s| {
            family.insert(0, s.to_string());
        });
    }
    fonts
        .families
        .insert(
            egui::FontFamily::Name("Bold".into()),
            vec![
                "Bold".to_owned(),
                "NotoBold".to_owned(),
                "NotoJPBold".to_owned(),
                "NotoKRBold".to_owned(),
                "NotoSCBold".to_owned(),
            ]
        );
    context.set_fonts(fonts);
}
