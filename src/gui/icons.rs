use egui::{Button, ImageButton, Response, TextureId, Ui, WidgetText};
use egui_extras::RetainedImage;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

static ICONS: OnceCell<FxHashMap<Icon, RetainedImage>> = OnceCell::new();

static ADD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>"#;
static ARROW_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>"#;
static ARROW_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="m4 12 1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z"/></svg>"#;
static BLANK: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#;
static CANCEL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path stroke="white" fill="white" d="m12.45 37.65-2.1-2.1L21.9 24 10.35 12.45l2.1-2.1L24 21.9l11.55-11.55 2.1 2.1L26.1 24l11.55 11.55-2.1 2.1L24 26.1Z"/></svg>"#;
static CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg>"#;
static DELETE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>"#;
static INFO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 0 24 24" width="24"><path d="M0 0h24v24H0z" fill="none"/><path stroke="white" fill="white" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>"#;
static FOLDER_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z"/></svg>"#;
static FOLDER_ZIP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm-2 6h-2v2h2v2h-2v2h-2v-2h2v-2h-2v-2h2v-2h-2V8h2v2h2v2z"/></svg>"#;
static FOLDER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>"#;
static MENU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/></svg>"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Add,
    ArrowBack,
    ArrowUp,
    Blank,
    Cancel,
    Check,
    Delete,
    Info,
    FolderOpen,
    FolderZip,
    Folder,
    Menu,
}

pub fn load_icons() {
    let mut map = FxHashMap::default();
    map.insert(
        Icon::Delete,
        RetainedImage::from_svg_str("delete", DELETE).unwrap(),
    );
    map.insert(
        Icon::Cancel,
        RetainedImage::from_svg_str("cancel", CANCEL).unwrap(),
    );
    map.insert(
        Icon::Menu,
        RetainedImage::from_svg_str("menu", MENU).unwrap(),
    );
    map.insert(
        Icon::Check,
        RetainedImage::from_svg_str("check", CHECK).unwrap(),
    );
    map.insert(Icon::Add, RetainedImage::from_svg_str("add", ADD).unwrap());
    map.insert(
        Icon::Info,
        RetainedImage::from_svg_str("info", INFO).unwrap(),
    );
    map.insert(
        Icon::Blank,
        RetainedImage::from_svg_str("blank", BLANK).unwrap(),
    );
    map.insert(
        Icon::Folder,
        RetainedImage::from_svg_str("folder", FOLDER).unwrap(),
    );
    map.insert(
        Icon::FolderZip,
        RetainedImage::from_svg_str("archive", FOLDER_ZIP).unwrap(),
    );
    map.insert(
        Icon::FolderOpen,
        RetainedImage::from_svg_str("folder-open", FOLDER_OPEN).unwrap(),
    );
    map.insert(
        Icon::ArrowUp,
        RetainedImage::from_svg_str("up", ARROW_UP).unwrap(),
    );
    map.insert(
        Icon::ArrowBack,
        RetainedImage::from_svg_str("back", ARROW_BACK).unwrap(),
    );
    unsafe { ICONS.set(map).unwrap_unchecked() }
}

#[inline(always)]
pub fn get_icon(ctx: &egui::Context, icon: Icon) -> TextureId {
    unsafe { ICONS.get_unchecked().get(&icon).unwrap_unchecked() }.texture_id(ctx)
}

pub trait IconButtonExt {
    fn icon_button(&mut self, icon: Icon) -> Response;
    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon: Icon) -> Response;
}

impl IconButtonExt for Ui {
    fn icon_button(&mut self, icon: Icon) -> Response {
        let button_padding = self.spacing().button_padding;
        self.spacing_mut().button_padding = button_padding / 2.;
        let res = self.add(
            ImageButton::new(
                get_icon(self.ctx(), icon),
                [self.spacing().icon_width, self.spacing().icon_width],
            )
            .tint(self.style().visuals.widgets.inactive.fg_stroke.color),
        );
        self.spacing_mut().button_padding = button_padding;
        res
    }

    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon: Icon) -> Response {
        self.add(Button::image_and_text(
            get_icon(self.ctx(), icon),
            [self.spacing().icon_width, self.spacing().icon_width],
            text,
        ))
    }
}
