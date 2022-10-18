use crate::icons::IconButtonExt;
use egui::{mutex::RwLock, Align, DragValue, Id, Layout, Response, Ui};
use egui_extras::Size;
use rustc_hash::FxHashSet;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::DerefMut,
    sync::Arc,
};

pub trait EditableValue {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response;
}

impl EditableValue for bool {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        ui.checkbox(self, "")
    }
}

macro_rules! impl_num {
    ($num:tt) => {
        impl EditableValue for $num {
            fn edit_ui(&mut self, ui: &mut Ui) -> Response {
                ui.add(DragValue::new(self))
            }
        }
    };
}

impl_num!(usize);
impl_num!(u8);
impl_num!(u16);
impl_num!(u32);
impl_num!(u64);
impl_num!(isize);
impl_num!(i8);
impl_num!(i16);
impl_num!(i32);
impl_num!(i64);
impl_num!(f32);
impl_num!(f64);

impl EditableValue for String {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        ui.text_edit_singleline(self)
    }
}

struct SmartStringWrapper<'a>(&'a mut smartstring::alias::String);

impl egui::TextBuffer for SmartStringWrapper<'_> {
    #[inline]
    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn is_mutable(&self) -> bool {
        true
    }

    #[inline]
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let index = self.byte_index_from_char_index(char_index);
        self.0.insert_str(index, text);
        text.chars().count()
    }

    #[inline]
    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        assert!(char_range.start <= char_range.end);
        let start = self.byte_index_from_char_index(char_range.start);
        let end = self.byte_index_from_char_index(char_range.end);
        self.0.drain(start..end);
    }
}

impl EditableValue for smartstring::alias::String {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut text = SmartStringWrapper(self);
        ui.text_edit_singleline(&mut text)
    }
}

impl<T> EditableValue for [T]
where
    T: EditableValue,
{
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let mut res = if self.len() < 5 {
            ui.horizontal(|ui| {
                self.iter_mut().for_each(|v| {
                    changed = changed || v.edit_ui(ui).changed();
                    ui.separator();
                });
            })
        } else {
            ui.group(|ui| {
                self.iter_mut().for_each(|v| {
                    changed = changed || v.edit_ui(ui).changed();
                    ui.separator();
                });
            })
        };
        if changed {
            res.response.mark_changed();
        }
        res.response
    }
}

impl EditableValue for Vec<u8> {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut text = hex::encode(self.as_slice());
        let res = ui.text_edit_singleline(&mut text);
        if res.changed() && let Ok(data) = hex::decode(text) {
            *self = data
        }
        res
    }
}

impl EditableValue for roead::byml::Byml {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        match self {
            roead::byml::Byml::String(v) => v.edit_ui(ui),
            roead::byml::Byml::BinaryData(v) => v.edit_ui(ui),
            roead::byml::Byml::Array(v) => v.edit_ui(ui),
            roead::byml::Byml::Hash(v) => v.edit_ui(ui),
            roead::byml::Byml::Bool(v) => v.edit_ui(ui),
            roead::byml::Byml::I32(v) => v.edit_ui(ui),
            roead::byml::Byml::Float(v) => v.edit_ui(ui),
            roead::byml::Byml::U32(v) => v.edit_ui(ui),
            roead::byml::Byml::I64(v) => v.edit_ui(ui),
            roead::byml::Byml::U64(v) => v.edit_ui(ui),
            roead::byml::Byml::Double(v) => v.edit_ui(ui),
            roead::byml::Byml::Null => ui.label("NULL"),
        }
    }
}

impl EditableValue for roead::byml::Hash {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut hasher = DefaultHasher::new();
        self.iter().for_each(|(k, v)| {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        });
        let hash = hasher.finish();
        let id = ui.make_persistent_id(hash);
        // egui::Grid::new(id)
        // .num_columns(2)
        // .striped(true)
        egui::Frame::none()
            .show(ui, |ui| {
                let mut dels: FxHashSet<smartstring::alias::String> = FxHashSet::default();
                ui.vertical(|ui| {
                    self.iter_mut().for_each(|(k, v)| {
                        egui_extras::StripBuilder::new(ui)
                            .size(Size::Absolute {
                                initial: 100.0,
                                range: (30.0, 300.0),
                            })
                            .size(Size::remainder())
                            .horizontal(|mut strip| {
                                strip.cell(|ui| {});
                                strip.cell(|ui| {});
                            });
                    });
                });
                dels.into_iter().for_each(|d| {
                    self.remove(&d);
                });
                ui.horizontal(|ui| {
                    let add_id = Id::new(hash + 1);
                    if ui.icon_button(crate::icons::Icon::Add).clicked() {
                        ui.memory()
                            .data
                            .insert_temp(add_id, Arc::new(RwLock::new(String::new())));
                    }
                    let mut added = false;
                    let new_key = ui
                        .ctx()
                        .memory()
                        .data
                        .get_temp::<Arc<RwLock<String>>>(add_id);
                    if let Some(key) = new_key {
                        let res = ui.text_edit_singleline(key.write().deref_mut());
                        if (res.has_focus() && ui.input().key_pressed(egui::Key::Enter))
                            || res.lost_focus()
                        {
                            added = true;
                        }
                    }
                    if added {
                        self.insert(
                            ui.memory()
                                .data
                                .get_temp::<Arc<RwLock<String>>>(add_id)
                                .expect("Missing new key name")
                                .read()
                                .as_str()
                                .into(),
                            roead::byml::Byml::String("".into()),
                        );
                        ui.memory().data.remove::<Arc<RwLock<String>>>(add_id);
                    }
                });
            })
            .response
    }
}
