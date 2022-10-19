use super::EditableValue;
use egui::{Id, Layout};
use roead::{
    aamp::{Parameter, ParameterIO, ParameterList, ParameterObject},
    types::{Color, Quat, Vector2f, Vector3f, Vector4f},
};

macro_rules! impl_edit_veclike {
    ($type:tt, $($field:ident),+) => {
        impl EditableValue for $type {
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
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.text_edit_singleline(&mut FixedSafeStringWrapper(self))
    }
}

impl EditableValue for Parameter {
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
            Parameter::BufferBinary(v) => v.edit_ui(ui),
            Parameter::StringRef(v) => v.edit_ui(ui),
        }
    }
}

impl EditableValue for ParameterObject {
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(
            ui,
            self.0.keys().next().map(|k| k.hash()).unwrap_or(946438626),
        )
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let table = roead::aamp::get_default_name_table();
        let id = Id::new(id);
        let mut changed = false;
        let mut res = egui::Grid::new(id.with("grid"))
            .num_columns(2)
            .min_col_width(30.0)
            .striped(true)
            .show(ui, |ui| {
                self.0.iter_mut().enumerate().for_each(|(i, (k, v))| {
                    ui.label(
                        table
                            .get_name(k.hash(), i, 0)
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| i.to_string()),
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
}

impl EditableValue for ParameterList {
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(
            ui,
            self.lists
                .0
                .keys()
                .next()
                .map(|k| k.hash())
                .unwrap_or(2767637356),
        )
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let table = roead::aamp::get_default_name_table();
        let id = Id::new(id);
        let mut changed = false;
        let mut res = egui::Frame::none()
            .show(ui, |ui| {
                egui::CollapsingHeader::new("Objects")
                    .id_source(id.with("objects"))
                    .show(ui, |ui| {
                        self.objects
                            .iter_mut()
                            .enumerate()
                            .for_each(|(i, (key, val))| {
                                let header = table
                                    .get_name(key.hash(), i, 0)
                                    .map(|k| k.to_string())
                                    .unwrap_or_else(|| key.hash().to_string());
                                egui::CollapsingHeader::new(header)
                                    .id_source(id.with(key))
                                    .show(ui, |ui| {
                                        changed = changed || val.edit_ui_with_id(ui, key).changed();
                                    });
                            });
                    });
                egui::CollapsingHeader::new("Lists")
                    .id_source(id.with("lists"))
                    .show(ui, |ui| {
                        self.lists
                            .iter_mut()
                            .enumerate()
                            .for_each(|(i, (key, val))| {
                                let header = table
                                    .get_name(key.hash(), i, 0)
                                    .map(|k| k.to_string())
                                    .unwrap_or_else(|| key.hash().to_string());
                                egui::CollapsingHeader::new(header)
                                    .id_source(id.with(key))
                                    .show(ui, |ui| {
                                        changed = changed || val.edit_ui_with_id(ui, key).changed();
                                    });
                            });
                    });
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}

impl EditableValue for ParameterIO {
    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let mut changed = false;
        let mut res = egui::Frame::none()
            .show(ui, |ui| {
                egui::Grid::new("pio_meta")
                    .num_columns(2)
                    .min_col_width(30.0)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("version");
                        changed = changed || self.version.edit_ui(ui).changed();
                        ui.end_row();
                        ui.label("type");
                        changed = changed || self.data_type.edit_ui(ui).changed();
                        ui.end_row()
                    });
                egui::CollapsingHeader::new("param_root").show(ui, |ui| {
                    changed =
                        changed || self.param_root.edit_ui_with_id(ui, "param_root").changed();
                });
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}
