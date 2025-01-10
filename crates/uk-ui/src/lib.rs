pub mod ext;
pub mod icons;
mod paths;
pub mod syntect;
pub mod visuals;
pub use egui;
pub use egui_dock;
pub use egui_extras;
use font_loader::system_fonts::FontPropertyBuilder;
pub use paths::PathNode;

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
            egui::FontData::from_owned(system_font.0),
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
            .insert("Bold".to_owned(), egui::FontData::from_owned(system_font.0));
    }
    fonts.font_data.insert(
        "Noto".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSans-Regular.ttf"))
    );
    fonts.font_data.insert(
        "NotoBold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSans-Bold.ttf"))
    );
    fonts.font_data.insert(
        "NotoJP".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansJP-Regular.ttf"))
    );
    fonts.font_data.insert(
        "NotoJPBold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansJP-Bold.ttf"))
    );
    fonts.font_data.insert(
        "NotoKR".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansKR-Regular.ttf"))
    );
    fonts.font_data.insert(
        "NotoKRBold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansKR-Bold.ttf"))
    );
    fonts.font_data.insert(
        "NotoSC".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansSC-Regular.ttf"))
    );
    fonts.font_data.insert(
        "NotoSCBold".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../fonts/NotoSansSC-Bold.ttf"))
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
