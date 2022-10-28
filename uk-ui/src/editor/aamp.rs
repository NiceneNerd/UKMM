use super::{EditableDisplay, EditableValue};
use crate::{icons::IconButtonExt, visuals};
use egui::{mutex::RwLock, Align, Id, Layout};
use roead::{
    aamp::{
        Parameter, ParameterIO, ParameterList, ParameterListMap, ParameterObject,
        ParameterObjectMap,
    },
    types::{Color, Quat, Vector2f, Vector3f, Vector4f},
};
use std::{ops::DerefMut, sync::Arc};

macro_rules! impl_edit_veclike {
    ($type:tt, $($field:ident),+) => {
        impl EditableValue for $type {
            const DISPLAY: EditableDisplay = EditableDisplay::Inline;
            fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
                ui.horizontal_wrapped(|ui| {
                    $(
                        ui.label(stringify!($field));
                        self.$field.edit_ui(ui);
                    )+
                }).response
            }
        }
    };
}

impl_edit_veclike!(Vector2f, x, y);
impl_edit_veclike!(Vector3f, x, y, z);
impl_edit_veclike!(Vector4f, x, y, z, t);
impl_edit_veclike!(Quat, a, b, c, d);

impl EditableValue for Color {
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut color = [self.r, self.g, self.b, self.a];
        let res = ui.color_edit_button_rgba_premultiplied(&mut color);
        if res.changed() {
            self.r = color[0];
            self.g = color[1];
            self.b = color[2];
            self.a = color[3];
        }
        res
    }
}

#[repr(transparent)]
struct FixedSafeStringWrapper<'a, const N: usize>(&'a mut roead::types::FixedSafeString<N>);

impl<const N: usize> egui::TextBuffer for FixedSafeStringWrapper<'_, N> {
    #[inline]
    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn is_mutable(&self) -> bool {
        true
    }

    #[inline]
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let mut string = self.0.to_string();
        let len = string.insert_text(text, char_index);
        *self.0 = string.as_str().into();
        len
    }

    #[inline]
    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        assert!(char_range.start <= char_range.end);
        let start = self.byte_index_from_char_index(char_range.start);
        let end = self.byte_index_from_char_index(char_range.end);
        let mut string = self.0.to_string();
        string.delete_char_range(start..end);
        *self.0 = string.as_str().into();
    }
}

impl<const N: usize> EditableValue for roead::types::FixedSafeString<N> {
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.text_edit_singleline(&mut FixedSafeStringWrapper(self))
    }
}

impl EditableValue for Parameter {
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        match self {
            Parameter::Bool(v) => v.edit_ui(ui),
            Parameter::F32(v) => v.edit_ui(ui),
            Parameter::Int(v) => v.edit_ui(ui),
            Parameter::Vec2(v) => v.edit_ui(ui),
            Parameter::Vec3(v) => v.edit_ui(ui),
            Parameter::Vec4(v) => v.edit_ui(ui),
            Parameter::Color(v) => v.edit_ui(ui),
            Parameter::String32(v) => v.edit_ui(ui),
            Parameter::String64(v) => v.edit_ui(ui),
            Parameter::Curve1(_) => todo!(),
            Parameter::Curve2(_) => todo!(),
            Parameter::Curve3(_) => todo!(),
            Parameter::Curve4(_) => todo!(),
            Parameter::BufferInt(_) => unimplemented!(),
            Parameter::BufferF32(_) => unimplemented!(),
            Parameter::BufferU32(_) => unimplemented!(),
            Parameter::String256(v) => v.edit_ui(ui),
            Parameter::Quat(v) => v.edit_ui(ui),
            Parameter::U32(v) => v.edit_ui(ui),
            Parameter::BufferBinary(v) => {
                let mut text = hex::encode(v.as_slice());
                let res = ui.text_edit_singleline(&mut text);
                if res.changed() && let Ok(data) = hex::decode(text) {
                    *v = data
                }
                res
            }
            Parameter::StringRef(v) => v.edit_ui(ui),
        }
    }
}

const PARAM_ROOT_HASH: u32 = 2767637356;

fn edit_ui_pobj(
    pobj: &mut ParameterObject,
    ui: &mut egui::Ui,
    parent: Option<u32>,
) -> egui::Response {
    let table = roead::aamp::get_default_name_table();
    let id = Id::new(
        parent
            .or_else(|| pobj.0.keys().next().map(|k| k.hash()))
            .unwrap_or(PARAM_ROOT_HASH),
    );
    let mut changed = false;
    let mut res = egui::Grid::new(id.with("grid"))
        .num_columns(2)
        .min_col_width(30.0)
        .striped(true)
        .show(ui, |ui| {
            pobj.0.iter_mut().enumerate().for_each(|(i, (k, v))| {
                ui.label(
                    table
                        .get_name(k.hash(), i, parent.unwrap_or(0))
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| k.hash().to_string()),
                );
                changed = changed || v.edit_ui(ui).changed();
                ui.end_row();
            });
        })
        .response;
    if changed {
        res.mark_changed();
    }
    res
}

impl EditableValue for ParameterObject {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    #[inline]
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        edit_ui_pobj(self, ui, None)
    }

    #[inline]
    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, _id: impl std::hash::Hash) -> egui::Response {
        edit_ui_pobj(self, ui, None)
    }
}

fn edit_ui_pobj_map(
    pobj_map: &mut ParameterObjectMap,
    ui: &mut egui::Ui,
    id: Id,
    parent: Option<u32>,
) -> bool {
    let table = roead::aamp::get_default_name_table();
    let mut changed = false;
    pobj_map.iter_mut().enumerate().for_each(|(i, (key, val))| {
        let header = table
            .get_name(key.hash(), i, parent.unwrap_or(0))
            .map(|k| k.to_string())
            .unwrap_or_else(|| key.hash().to_string());
        egui::CollapsingHeader::new(header)
            .id_source(id.with(key))
            .show(ui, |ui| {
                changed = changed || edit_ui_pobj(val, ui, Some(key.hash())).changed();
            });
    });
    changed
}

impl EditableValue for ParameterObjectMap {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "pobj_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let mut changed = false;
        let mut res = ui
            .scope(|ui| {
                changed = edit_ui_pobj_map(self, ui, Id::new(id), None);
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

fn edit_ui_plist_map(
    plist_map: &mut ParameterListMap,
    ui: &mut egui::Ui,
    id: Id,
    parent: Option<u32>,
) -> bool {
    let table = roead::aamp::get_default_name_table();
    let mut changed = false;
    plist_map
        .iter_mut()
        .enumerate()
        .for_each(|(i, (key, val))| {
            let header = table
                .get_name(key.hash(), i, parent.unwrap_or(0))
                .map(|k| k.to_string())
                .unwrap_or_else(|| key.hash().to_string());
            egui::CollapsingHeader::new(header)
                .id_source(id.with(key))
                .show(ui, |ui| {
                    changed = changed || edit_ui_plist(val, ui, Some(key.hash())).changed();
                });
        });
    changed
}

impl EditableValue for ParameterListMap {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "plist_map")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let mut changed = false;
        let mut res = ui
            .scope(|ui| {
                changed = edit_ui_plist_map(self, ui, Id::new(id), None);
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

fn edit_ui_plist(
    plist: &mut ParameterList,
    ui: &mut egui::Ui,
    parent: Option<u32>,
) -> egui::Response {
    let id = Id::new(
        parent
            .or_else(|| plist.lists.0.keys().next().map(|k| k.hash()))
            .unwrap_or(PARAM_ROOT_HASH),
    );
    let mut changed = false;
    let mut res = egui::Frame::none()
        .show(ui, |ui| {
            egui::CollapsingHeader::new("objects")
                .id_source(id.with("objects"))
                .show(ui, |ui| {
                    changed = changed || edit_ui_pobj_map(&mut plist.objects, ui, id, parent);
                });
            egui::CollapsingHeader::new("lists")
                .id_source(id.with("lists"))
                .show(ui, |ui| {
                    changed = changed || edit_ui_plist_map(&mut plist.lists, ui, id, parent);
                });
        })
        .response;
    if changed {
        res.mark_changed();
    }
    res
}

impl EditableValue for ParameterList {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        edit_ui_plist(self, ui, None)
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, _id: impl std::hash::Hash) -> egui::Response {
        edit_ui_plist(self, ui, None)
    }
}

fn edit_pio_code(pio: &mut ParameterIO, ui: &mut egui::Ui, id: Id) -> egui::Response {
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let yaml = ui
            .data()
            .get_temp_mut_or_insert_with(id, || Arc::new(RwLock::new(pio.to_text())))
            .clone();
        ui.allocate_ui_with_layout(
            [
                ui.spacing().icon_width + ui.spacing().item_spacing.x * 2.0,
                ui.available_height(),
            ]
            .into(),
            Layout::top_down(Align::Center),
            |ui| {
                if ui
                    .icon_button(crate::icons::Icon::Check)
                    .on_hover_text("Save")
                    .clicked()
                {
                    match roead::aamp::ParameterIO::from_text(yaml.read().as_str()) {
                        Ok(val) => {
                            ui.memory()
                                .data
                                .insert_temp::<bool>(id.with("error"), false);
                            *pio = val;
                        }
                        Err(_) => ui.memory().data.insert_temp(id.with("error"), true),
                    }
                }
                if ui
                    .icon_button(crate::icons::Icon::Cancel)
                    .on_hover_text("Reset")
                    .clicked()
                {
                    *yaml.write() = pio.to_text();
                    ui.memory()
                        .data
                        .insert_temp::<bool>(id.with("error"), false);
                }
            },
        );
        let has_err = ui.memory().data.get_temp(id.with("error")).unwrap_or(false);
        if has_err {
            ui.visuals_mut().extreme_bg_color = visuals::error_bg(ui.visuals());
        }
        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = crate::syntect::highlight(
                ui.ctx(),
                &crate::syntect::CodeTheme::dark(),
                string,
                "yaml",
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts().layout_job(layout_job)
        };
        let res = egui::TextEdit::multiline(yaml.write().deref_mut())
            .layouter(&mut layouter)
            .code_editor()
            .desired_width(ui.available_width())
            .show(ui);
        if has_err {
            res.response.on_hover_text_at_pointer("Invalid YAML")
        } else {
            res.response
        }
    })
    .response
}

impl EditableValue for ParameterIO {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;
        let code_flag_id = Id::new("pio")
            .with(self.data_type.as_str())
            .with(self.version)
            .with("code");
        let mut code_editor = *ui.data().get_temp_mut_or_default::<bool>(code_flag_id);
        let mut res = ui
            .vertical_centered_justified(|ui| {
                egui::Grid::new("pio_meta")
                    .min_col_width(30.0)
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.toggle_value(&mut code_editor, "Code").changed() {
                                ui.data().insert_temp(code_flag_id, code_editor);
                                if !code_editor {
                                    let text = ui
                                        .data()
                                        .get_temp::<Arc<RwLock<String>>>(Id::new("pio-code"))
                                        .expect("YAML text should exist");
                                    let text = text.read();
                                    if let Ok(pio) = ParameterIO::from_text(text.as_str()) {
                                        *self = pio
                                    }
                                }
                            }
                            ui.allocate_space([ui.available_width(), 0.0].into());
                        });
                        ui.end_row();
                        if !code_editor {
                            ui.label("version");
                            ui.horizontal(|ui| {
                                changed = changed || self.version.edit_ui(ui).changed();
                            });
                            ui.end_row();
                            ui.label("type");
                            ui.horizontal(|ui| {
                                changed = changed || self.data_type.edit_ui(ui).changed();
                            });
                            ui.end_row()
                        }
                    });
                if code_editor {
                    edit_pio_code(self, ui, Id::new("pio-code"));
                } else {
                    egui::CollapsingHeader::new("param_root").show(ui, |ui| {
                        changed = changed
                            || edit_ui_plist(&mut self.param_root, ui, Some(PARAM_ROOT_HASH))
                                .changed();
                    });
                }
            })
            .response;
        if changed {
            res.mark_changed();
        }
        ui.allocate_space(ui.available_size());
        res
    }
}
