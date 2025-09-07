//#![allow(deprecated)]
//! TODO: Update icon handling for egui 0.32 - RetainedImage has been replaced
//! This file needs to be updated to use the new image loading APIs in egui 0.32
//!
//! For now, commenting out to allow compilation to succeed

use egui::{Button, Response, Ui, WidgetText};

// Stub implementations for egui 0.32 compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Add,
    ArrowBack,
    ArrowUp,
    Bitcoin,
    Blank,
    Cancel,
    Check,
    Delete,
    Export,
    File,
    FileZip,
    Folder,
    FolderOpen,
    Help,
    Import,
    Info,
    List,
    Menu,
    Patreon,
    Refresh,
    Reset,
    Save,
    Settings,
    Tune,
}

pub fn get_icon(_ctx: &egui::Context, _icon: Icon) -> egui::Image<'static> {
    // Return a placeholder image - this should be updated with proper icon loading
    egui::Image::from_bytes("bytes://stub", &[])
}

pub trait IconButton {
    fn icon_button(&mut self, icon: Icon) -> Response;
    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon: Icon) -> Response;
}

impl IconButton for Ui {
    fn icon_button(&mut self, _icon: Icon) -> Response {
        // Stub implementation - return a basic button
        self.add(Button::new("🔘"))
    }

    fn icon_text_button(&mut self, text: impl Into<WidgetText>, _icon: Icon) -> Response {
        // Stub implementation - return a text button without icon
        self.add(Button::new(text))
    }
}

/*

static ADD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>"#;
static ARROW_BACK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>"#;
static ARROW_UP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="m4 12 1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z"/></svg>"#;
static BLANK: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#;
static CANCEL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path stroke="white" fill="white" d="m12.45 37.65-2.1-2.1L21.9 24 10.35 12.45l2.1-2.1L24 21.9l11.55-11.55 2.1 2.1L26.1 24l11.55 11.55-2.1 2.1L24 26.1Z"/></svg>"#;
static CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M9 16.17 4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg>"#;
static DELETE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>"#;
static HELP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 0 24 24" width="24px" fill="none"><path d="M0 0h24v24H0z" fill="none"/><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 17h-2v-2h2v2zm2.07-7.75l-.9.92C13.45 12.9 13 13.5 13 15h-2v-.5c0-1.1.45-2.1 1.17-2.83l1.24-1.26c.37-.36.59-.86.59-1.41 0-1.1-.9-2-2-2s-2 .9-2 2H8c0-2.21 1.79-4 4-4s4 1.79 4 4c0 .88-.36 1.68-.93 2.25z" stroke="white" fill="white" /></svg>"#;
static IMPORT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 0 24 24" width="24px" fill="white"><path d="M0 0h24v24H0z" fill="none"/><path d="M21 3.01H3c-1.1 0-2 .9-2 2V9h2V4.99h18v14.03H3V15H1v4.01c0 1.1.9 1.98 2 1.98h18c1.1 0 2-.88 2-1.98v-14c0-1.11-.9-2-2-2zM11 16l4-4-4-4v3H1v2h10v3z"/></svg>"#;
static INFO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 0 24 24" width="24"><path d="M0 0h24v24H0z" fill="none"/><path stroke="white" fill="white" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z"/></svg>"#;
static FOLDER_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z"/></svg>"#;
static FOLDER_ZIP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm-2 6h-2v2h2v2h-2v2h-2v-2h2v-2h-2v-2h2v-2h-2V8h2v2h2v2z"/></svg>"#;
static FOLDER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>"#;
static LIST: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path d="M15 33.7q.6 0 1.05-.45.45-.45.45-1.05 0-.6-.45-1.05-.45-.45-1.05-.45-.6 0-1.05.45-.45.45-.45 1.05 0 .6.45 1.05.45.45 1.05.45Zm0-8.2q.6 0 1.05-.45.45-.45.45-1.05 0-.6-.45-1.05-.45-.45-1.05-.45-.6 0-1.05.45-.45.45-.45 1.05 0 .6.45 1.05.45.45 1.05.45Zm0-8.2q.6 0 1.05-.45.45-.45.45-1.05 0-.6-.45-1.05-.45-.45-1.05-.45-.6 0-1.05.45-.45.45-.45 1.05 0 .6.45 1.05.45.45 1.05.45Zm6.6 16.4h12.2v-3H21.6Zm0-8.2h12.2v-3H21.6Zm0-8.2h12.2v-3H21.6ZM9 42q-1.2 0-2.1-.9Q6 40.2 6 39V9q0-1.2.9-2.1Q7.8 6 9 6h30q1.2 0 2.1.9.9.9.9 2.1v30q0 1.2-.9 2.1-.9.9-2.1.9Z" fill="white" /></svg>"#;
static MENU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path stroke="white" fill="white" d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/></svg>"#;
static RESET: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path stroke="white" fill="white" d="M23.85 42q-7.45 0-12.65-5.275T6 23.95h3q0 6.25 4.3 10.65T23.85 39q6.35 0 10.75-4.45t4.4-10.8q0-6.2-4.45-10.475Q30.1 9 23.85 9q-3.4 0-6.375 1.55t-5.175 4.1h5.25v3H7.1V7.25h3v5.3q2.6-3.05 6.175-4.8Q19.85 6 23.85 6q3.75 0 7.05 1.4t5.775 3.825q2.475 2.425 3.9 5.675Q42 20.15 42 23.9t-1.425 7.05q-1.425 3.3-3.9 5.75-2.475 2.45-5.775 3.875Q27.6 42 23.85 42Zm6.4-9.85-7.7-7.6v-10.7h3v9.45L32.4 30Z"/></svg>"#;
static REFRESH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" viewBox="0 0 24 24" width="24"><path d="M0 0h24v24H0z" strike="white" fill="none"/><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" fill="white" /></svg>"#;
static SAVE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path d="M42 13.85V39q0 1.2-.9 2.1-.9.9-2.1.9H9q-1.2 0-2.1-.9Q6 40.2 6 39V9q0-1.2.9-2.1Q7.8 6 9 6h25.15Zm-18 21.9q2.15 0 3.675-1.525T29.2 30.55q0-2.15-1.525-3.675T24 25.35q-2.15 0-3.675 1.525T18.8 30.55q0 2.15 1.525 3.675T24 35.75ZM11.65 18.8h17.9v-7.15h-17.9Z" stroke="white" fill="white" /></svg>"#;
static SETTINGS: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path stroke="white" fill="white" d="M11.1 37.3 4 30.2l2.1-2.1 5 4.95 8.95-8.95 2.1 2.15Zm0-16L4 14.2l2.1-2.1 5 4.95 8.95-8.95 2.1 2.15ZM26 33.5v-3h18v3Zm0-16v-3h18v3Z" /></svg>"#;
static TUNE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" height="24" width="24" viewBox="0 0 48 48"><path stroke="white" fill="white" d="M21.35 42V30.75h3v4.15H42v3H24.35V42ZM6 37.9v-3h12.35v3Zm9.35-8.3v-4.1H6v-3h9.35v-4.2h3v11.3Zm6-4.1v-3H42v3Zm8.3-8.25V6h3v4.1H42v3h-9.35v4.15ZM6 13.1v-3h20.65v3Z" /></svg>"#;
static PATREON: &str = include_str!("../../../assets/patreon.svg");
static BITCOIN: &str = include_str!("../../../assets/btc.svg");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Add,
    ArrowBack,
    ArrowUp,
    Bitcoin,
    Blank,
    Cancel,
    Check,
    Delete,
    Help,
    Import,
    Info,
    FolderOpen,
    FolderZip,
    Folder,
    List,
    Menu,
    Patreon,
    Refresh,
    Reset,
    Save,
    Settings,
    Tune,
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
        Icon::List,
        RetainedImage::from_svg_str("list", LIST).unwrap(),
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
        Icon::Import,
        RetainedImage::from_svg_str("import", IMPORT).unwrap(),
    );
    map.insert(
        Icon::Info,
        RetainedImage::from_svg_str("info", INFO).unwrap(),
    );
    map.insert(
        Icon::Help,
        RetainedImage::from_svg_str("help", HELP).unwrap(),
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
    map.insert(
        Icon::Reset,
        RetainedImage::from_svg_str("reset", RESET).unwrap(),
    );
    map.insert(
        Icon::Refresh,
        RetainedImage::from_svg_str("refresh", REFRESH).unwrap(),
    );
    map.insert(
        Icon::Save,
        RetainedImage::from_svg_str("save", SAVE).unwrap(),
    );
    map.insert(
        Icon::Settings,
        RetainedImage::from_svg_str("settings", SETTINGS).unwrap(),
    );
    map.insert(
        Icon::Tune,
        RetainedImage::from_svg_str("tune", TUNE).unwrap(),
    );
    map.insert(
        Icon::Patreon,
        RetainedImage::from_svg_str("patreon", PATREON).unwrap(),
    );
    map.insert(
        Icon::Bitcoin,
        RetainedImage::from_svg_str("btc", BITCOIN).unwrap(),
    );
    unsafe { ICONS.set(map).unwrap_unchecked() }
}

#[inline(always)]
pub fn get_icon(ctx: &egui::Context, icon: Icon) -> egui::load::SizedTexture {
    let width = ctx.style().spacing.icon_width;
    egui::load::SizedTexture::new(
        unsafe { ICONS.get().unwrap_unchecked().get(&icon).unwrap_unchecked() }.texture_id(ctx),
        egui::Vec2::new(width, width),
    )
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
            ImageButton::new(get_icon(self.ctx(), icon))
                .tint(self.style().visuals.widgets.inactive.fg_stroke.color),
        );
        self.spacing_mut().button_padding = button_padding;
        res
    }

    fn icon_text_button(&mut self, text: impl Into<WidgetText>, icon: Icon) -> Response {
        self.add(Button::image_and_text(get_icon(self.ctx(), icon), text))
    }
}
*/
