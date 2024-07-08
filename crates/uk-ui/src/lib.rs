pub mod editor;
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

// 自定义字体
pub fn insert_custom_fonts(fonts: &mut egui::FontDefinitions) {
    // 安装的字体支持.ttf和.otf文件
    // 文件放在main.rs的同级目录下（可以自定义到其它目录）
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../assets/ZHcn.ttf")),
    );
    // 将字体添加到 Proportional 字体族的第一个位置
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());
    // 将字体添加到 Monospace 字体族的末尾
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());
}

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
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "System".to_owned());
    fonts
        .families
        .insert(egui::FontFamily::Name("Bold".into()), vec![
            "Bold".to_owned(),
        ]);
        insert_custom_fonts(&mut fonts);
    context.set_fonts(fonts);
}
