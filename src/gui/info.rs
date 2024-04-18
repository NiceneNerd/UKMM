use std::{
    hash::{Hash, Hasher},
    io::{BufReader, Read},
    sync::Arc,
};

use anyhow::Result;
use nk_ui::{
    egui::{self, Align, Label, Layout, RichText, Ui},
    egui_extras::RetainedImage,
    icons::IconButtonExt,
    PathNode,
};
use nk_util::Lazy;
use parking_lot::{Mutex, RwLock};
use rustc_hash::{FxHashMap, FxHasher};
use uk_manager::mods::Mod;
use uk_mod::Manifest;

use super::Component;

pub enum Message {
    RequestOptions,
}

#[repr(transparent)]
pub struct ModInfo<'a>(pub &'a Mod);

impl ModInfo<'_> {
    pub fn preview(&self) -> Option<Arc<RetainedImage>> {
        fn load_preview(mod_: &Mod) -> Result<Option<Arc<RetainedImage>>> {
            let mut zip = zip::ZipArchive::new(BufReader::new(std::fs::File::open(&mod_.path)?))?;
            for ext in ["jpg", "jpeg", "png", "svg"] {
                if let Ok(mut file) = zip.by_name(&format!("thumb.{}", ext)) {
                    let mut vec = vec![0; file.size() as usize];
                    file.read_exact(&mut vec)?;
                    return Ok(Some(Arc::new(
                        RetainedImage::from_image_bytes(mod_.meta.name.as_str(), &vec)
                            .map_err(|e| anyhow::anyhow!("{}", e))?,
                    )));
                }
            }
            Ok(None)
        }
        static PREVIEW: Lazy<RwLock<FxHashMap<usize, Option<Arc<RetainedImage>>>>> =
            Lazy::new(|| RwLock::new(FxHashMap::default()));
        let mut preview = PREVIEW.write();
        preview
            .entry(self.0.hash())
            .or_insert_with(|| {
                match load_preview(self.0) {
                    Ok(pre) => pre,
                    Err(e) => {
                        log::error!("Error loading mod preview: {}", e);
                        None
                    }
                }
            })
            .clone()
    }
}

impl Component for ModInfo<'_> {
    type Message = Message;

    fn show(&self, ui: &mut Ui) -> egui::InnerResponse<Option<Self::Message>> {
        let mut msg = None;
        let mod_ = self.0;
        egui::Frame::none().inner_margin(2.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.;
            ui.add_space(8.);
            if let Some(preview) = self.preview() {
                preview.show_max_size(ui, ui.available_size());
                ui.add_space(8.);
            }
            let ver = mod_.meta.version.to_string();
            [
                ("Name", mod_.meta.name.as_str()),
                ("Version", ver.as_str()),
                ("Category", mod_.meta.category.as_str()),
                ("Author", mod_.meta.author.as_str()),
            ]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .for_each(|(label, value)| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).family(egui::FontFamily::Name("Bold".into())));
                    ui.add_space(8.);
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        ui.add(Label::new(value).wrap(true));
                    })
                });
            });
            ui.label(RichText::new("Description").family(egui::FontFamily::Name("Bold".into())));
            ui.add_space(4.);
            let md_cache = ui
                .data()
                .get_temp_mut_or_default::<Arc<Mutex<egui_commonmark::CommonMarkCache>>>(
                    egui::Id::new("md_cache"),
                )
                .clone();
            egui_commonmark::CommonMarkViewer::new("mod_description").show(
                ui,
                &mut md_cache.lock(),
                &mod_.meta.description,
            );
            ui.add_space(4.);
            if !mod_.meta.options.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Enabled Options")
                            .family(egui::FontFamily::Name("Bold".into())),
                    );
                    ui.add_space(8.);
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui.icon_button(nk_ui::icons::Icon::Settings).clicked() {
                            msg = Some(Message::RequestOptions);
                        }
                    })
                });
                ui.add_space(4.0);
                if !mod_.enabled_options.is_empty() {
                    ui.add_enabled_ui(false, |ui| {
                        mod_.enabled_options.iter().for_each(|opt| {
                            ui.checkbox(&mut true, opt.name.as_str());
                        });
                    });
                } else {
                    ui.label("No options enabled");
                }
                ui.add_space(4.0);
            }
            ui.label(RichText::new("Manifest").family(egui::FontFamily::Name("Bold".into())));
            match mod_.manifest() {
                Ok(manifest) => render_manifest(&manifest, ui),
                Err(e) => {
                    log::error!("{:#?}", e);
                    ui.label(RichText::new("FAILED TO LOAD MANIFEST").strong());
                }
            }
            ui.add_space(8.0);
            msg
        })
    }
}

pub fn render_manifest(manifest: &Manifest, ui: &mut Ui) {
    ui.scope(|ui| {
        static ROOTS: Lazy<RwLock<FxHashMap<u64, PathNode>>> =
            Lazy::new(|| RwLock::new(FxHashMap::default()));
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
        ui.spacing_mut().item_spacing.y = 4.;
        if !manifest.content_files.is_empty() {
            let mut hasher = FxHasher::default();
            manifest.content_files.hash(&mut hasher);
            let mut roots = ROOTS.write();
            let content_root = roots.entry(hasher.finish()).or_insert_with(|| {
                let mut root = PathNode::dir("Base Files");
                manifest.content_files.iter().for_each(|file| {
                    root.build_tree(&file.split('/').map(|s| s.to_owned()).collect(), 0);
                });
                root
            });
            content_root.render_dir(ui);
        }
        if !manifest.aoc_files.is_empty() {
            let mut hasher = FxHasher::default();
            manifest.aoc_files.hash(&mut hasher);
            let mut roots = ROOTS.write();
            let aoc_root = roots.entry(hasher.finish()).or_insert_with(|| {
                let mut root = PathNode::dir("DLC Files");
                manifest.aoc_files.iter().for_each(|file| {
                    root.build_tree(&file.split('/').map(|s| s.to_owned()).collect(), 0);
                });
                root
            });
            aoc_root.render_dir(ui);
        }
    });
}
