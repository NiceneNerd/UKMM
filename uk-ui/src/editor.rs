use egui::{DragValue, Response, Ui};
use std::hash::Hash;
pub mod aamp;
pub mod byml;

pub trait EditableValue {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response;
    fn edit_ui_with_id(&mut self, ui: &mut Ui, _id: impl Hash) -> Response {
        self.edit_ui(ui)
    }
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

impl<T: EditableValue + Default> EditableValue for Option<T> {
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut is_some = self.is_some();
        let mut changed = false;
        let mut res = ui.scope(|ui| match self.as_mut() {
            Some(val) => {
                if ui.checkbox(&mut is_some, "").changed() {
                    *self = None;
                    changed = true;
                } else {
                    changed = changed || val.edit_ui(ui).changed();
                }
            }
            None => {
                if ui.checkbox(&mut is_some, "").changed() {
                    *self = Self::default();
                    changed = true;
                }
            }
        });
        if changed {
            res.response.mark_changed();
        }
        res.response
    }
}
