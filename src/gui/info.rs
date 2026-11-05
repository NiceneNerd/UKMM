use std::{
    hash::{Hash, Hasher},
    io::{BufReader, Read},
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use rustc_hash::{FxHashMap, FxHasher};
use uk_localization::string_ext::LocString;
use uk_manager::mods::Mod;
use uk_mod::Manifest;
#[allow(deprecated)]
use uk_ui::{
    egui::{self, Align, Label, Layout, RichText, Ui},
    egui_extras::image,
    icons::IconButtonExt,
    PathNode,
};
use super::Component;

pub enum Message {
    RequestOptions,
}

#[repr(transparent)]
pub struct ModInfo<'a>(pub &'a Mod);

impl ModInfo<'_> {
    #[allow(deprecated)]
    pub fn preview(&self, ctx: &egui::Context) -> Option<Arc<egui::TextureHandle>> {
        fn load_preview(mod_: &Mod, ctx: &egui::Context) -> Result<Option<Arc<egui::TextureHandle>>> {
            let mut zip = zip::ZipArchive::new(BufReader::new(std::fs::File::open(&mod_.path)?))?;
            for ext in ["jpg", "jpeg", "png", "svg"] {
                if let Ok(mut file) = zip.by_name(&format!("thumb.{}", ext)) {
                    let mut vec = vec![0; file.size() as usize];
                    file.read_exact(&mut vec)?;
                    let image = image::load_image_bytes(&vec)
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    return Ok(Some(Arc::new(
                        ctx.load_texture(mod_.meta.name.as_str(), image, Default::default()),
                    )));
                }
            }
            Ok(None)
        }
        static PREVIEW: LazyLock<RwLock<FxHashMap<usize, Option<Arc<egui::TextureHandle>>>>> =
            LazyLock::new(|| RwLock::new(FxHashMap::default()));
        let mut preview = PREVIEW.write();
        preview
            .entry(self.0.hash())
            .or_insert_with(|| {
                load_preview(self.0, ctx).unwrap_or_else(|e| {
                    log::error!("Error loading mod preview: {}", e);
                    None
                })
            })
            .clone()
    }
}

impl Component for ModInfo<'_> {
    type Message = Message;

    fn show(&self, ui: &mut Ui) -> egui::InnerResponse<Option<Self::Message>> {
        let mut msg = None;
        let mod_ = self.0;
        egui::Frame::NONE.inner_margin(2.0).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.;
            ui.add_space(8.);
            if let Some(preview) = self.preview(ui.ctx()) {
                let available = ui.available_size();
                ui.add(egui::Image::from_texture(
                    egui::load::SizedTexture::new(preview.id(), available)
                ));
                //preview.show_max_size(ui, [available.x.max(0.0), available.y.max(0.0)].into());
                ui.add_space(8.);
            }
            let ver = mod_.meta.version.to_string();
            [
                ("Info_Name".localize(), mod_.meta.name.as_str()),
                ("Info_Version".localize(), ver.as_str()),
                ("Info_Category".localize(), mod_.meta.category.into()),
                ("Info_Author".localize(), mod_.meta.author.as_str()),
            ]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .for_each(|(label, value)| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).family(egui::FontFamily::Name("Bold".into())));
                    ui.add_space(8.);
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        ui.add(Label::new(value).wrap_mode(egui::TextWrapMode::Truncate));
                    })
                });
            });
            ui.label(RichText::new("Info_Description".localize())
                .family(egui::FontFamily::Name("Bold".into())));
            ui.add_space(4.);
            let md_cache = ui.data_mut(|d| {
                d.get_temp_mut_or_default::<Arc<Mutex<egui_commonmark::CommonMarkCache>>>(
                    egui::Id::new("md_cache"),
                )
                .clone()
            });
            egui_commonmark::CommonMarkViewer::new(/* "mod_description" */).show(
                ui,
                &mut md_cache.lock(),
                &mod_.meta.description,
            );
            ui.add_space(4.);
            if !mod_.meta.options.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Info_Options".localize())
                            .family(egui::FontFamily::Name("Bold".into())),
                    );
                    ui.add_space(8.);
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui.icon_button(uk_ui::icons::Icon::Settings).clicked() {
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
                    ui.label("Info_Options_None".localize());
                }
                ui.add_space(4.0);
            }
            ui.label(RichText::new("Info_Manifest".localize())
                .family(egui::FontFamily::Name("Bold".into())));
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

pub static ROOTS: LazyLock<RwLock<FxHashMap<u64, PathNode>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

pub fn render_manifest(manifest: &Manifest, ui: &mut Ui) {
    ui.scope(|ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
        ui.spacing_mut().item_spacing.y = 4.;
        if !manifest.content_files.is_empty() {
            let mut hasher = FxHasher::default();
            manifest.content_files.hash(&mut hasher);
            let mut roots = ROOTS.write();
            let content_root = roots.entry(hasher.finish()).or_insert_with(|| {
                let val = "Info_Manifest_BaseFiles".localize();
                let mut root = PathNode::dir(&val);
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
                let val = "Info_Manifest_DLCFiles".localize();
                let mut root = PathNode::dir(&val);
                manifest.aoc_files.iter().for_each(|file| {
                    root.build_tree(&file.split('/').map(|s| s.to_owned()).collect(), 0);
                });
                root
            });
            aoc_root.render_dir(ui);
        }
    });
}
