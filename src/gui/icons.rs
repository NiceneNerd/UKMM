use egui::{Button, Color32, ImageButton, Response, TextureHandle, TextureId, Ui, WidgetText};
use egui_extras::RetainedImage;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

static ICONS: OnceCell<FxHashMap<&'static str, RetainedImage>> = OnceCell::new();

static ADD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>"#;
static ARROW_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>"#;
static ARROW_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="m4 12 1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z"/></svg>"#;
static CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg>"#;
static DELETE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>"#;
static FOLDER_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z"/></svg>"#;
static FOLDER_ZIP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm-2 6h-2v2h2v2h-2v2h-2v-2h2v-2h-2v-2h2v-2h-2V8h2v2h2v2z"/></svg>"#;
static FOLDER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>"#;
static MENU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/></svg>"#;

pub fn load_icons() {
    let mut map = FxHashMap::default();
    map.insert(
        "delete",
        RetainedImage::from_svg_str("delete", DELETE).unwrap(),
    );
    map.insert("menu", RetainedImage::from_svg_str("menu", MENU).unwrap());
    map.insert(
        "check",
        RetainedImage::from_svg_str("check", CHECK).unwrap(),
    );
    map.insert("add", RetainedImage::from_svg_str("add", ADD).unwrap());
    map.insert(
        "folder",
        RetainedImage::from_svg_str("folder", FOLDER).unwrap(),
    );
    map.insert(
        "archive",
        RetainedImage::from_svg_str("archive", FOLDER_ZIP).unwrap(),
    );
    map.insert(
        "folder-open",
        RetainedImage::from_svg_str("folder-open", FOLDER_OPEN).unwrap(),
    );
    map.insert("up", RetainedImage::from_svg_str("up", ARROW_UP).unwrap());
    map.insert(
        "back",
        RetainedImage::from_svg_str("back", ARROW_BACK).unwrap(),
    );
    unsafe { ICONS.set(map).unwrap_unchecked() }
}

#[inline(always)]
pub fn get_icon(ctx: &egui::Context, icon: &str) -> TextureId {
    unsafe { ICONS.get_unchecked().get(icon).unwrap_unchecked() }.texture_id(ctx)
}

pub trait IconButtonExt {
    fn icon_button(&mut self, icon_name: &str) -> Response;
    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon_name: &str) -> Response;
}

impl IconButtonExt for Ui {
    fn icon_button(&mut self, icon_name: &str) -> Response {
        self.add(
            ImageButton::new(
                get_icon(self.ctx(), icon_name),
                [self.spacing().icon_width, self.spacing().icon_width],
            )
            .tint(self.style().visuals.widgets.inactive.fg_stroke.color),
        )
    }

    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon_name: &str) -> Response {
        self.add(Button::image_and_text(
            get_icon(self.ctx(), icon_name),
            [self.spacing().icon_width, self.spacing().icon_width],
            text,
        ))
    }
}
