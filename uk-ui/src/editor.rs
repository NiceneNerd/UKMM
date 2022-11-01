use egui::{DragValue, Id, Response, Ui};
use std::hash::Hash;
pub mod aamp;
pub mod byml;
pub mod maps;
pub mod msyt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditableDisplay {
    Block,
    Inline,
}

pub trait EditableValue {
    const DISPLAY: EditableDisplay;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response;
    fn edit_ui_with_id(&mut self, ui: &mut Ui, _id: impl Hash) -> Response {
        self.edit_ui(ui)
    }
}

impl EditableValue for bool {
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        ui.checkbox(self, if *self { "True" } else { "False" })
    }
}

macro_rules! impl_num {
    ($num:tt) => {
        impl EditableValue for $num {
            const DISPLAY: EditableDisplay = EditableDisplay::Inline;
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
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
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
    const DISPLAY: EditableDisplay = EditableDisplay::Inline;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut text = SmartStringWrapper(self);
        ui.text_edit_singleline(&mut text)
    }
}

impl<T> EditableValue for [T]
where
    T: EditableValue,
{
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let mut res = if self.len() < 5 {
            ui.horizontal(|ui| {
                self.iter_mut().for_each(|v| {
                    changed |= v.edit_ui(ui).changed();
                    ui.separator();
                });
            })
        } else {
            ui.group(|ui| {
                self.iter_mut().for_each(|v| {
                    changed |= v.edit_ui(ui).changed();
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

impl<T: EditableValue> EditableValue for Vec<T> {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        let mut changed = false;
        let mut res = if self.len() < 5 {
            ui.horizontal(|ui| {
                self.iter_mut().for_each(|v| {
                    changed |= v.edit_ui(ui).changed();
                    ui.separator();
                });
            })
        } else {
            ui.group(|ui| {
                self.iter_mut().for_each(|v| {
                    changed |= v.edit_ui(ui).changed();
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

impl<T: EditableValue + Default + PartialEq> EditableValue for Option<T> {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        self.edit_ui_with_id(ui, "option")
    }

    fn edit_ui_with_id(&mut self, ui: &mut Ui, id: impl Hash) -> Response {
        let mut changed = false;
        let id = Id::new(id).with("value");
        let mut res = ui.vertical(|ui| {
            ui.horizontal(|ui| {
                changed |= ui.radio_value(self, None, "None").clicked();
                if ui.radio(self.is_some(), "Set Value").clicked() {
                    *self = Some(T::default());
                    changed = true;
                }
            });
            if let Some(ref mut value) = self {
                changed |= value.edit_ui_with_id(ui, id).changed();
            }
        });
        if changed {
            res.response.mark_changed();
        }
        res.response
    }
}

impl<T: EditableValue> EditableValue for Box<T> {
    const DISPLAY: EditableDisplay = T::DISPLAY;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        self.edit_ui_with_id(ui, "boxed")
    }

    fn edit_ui_with_id(&mut self, ui: &mut Ui, id: impl Hash) -> Response {
        self.as_mut().edit_ui_with_id(ui, id)
    }
}

impl<T: EditableValue, U: EditableValue> EditableValue for (T, U) {
    const DISPLAY: EditableDisplay = T::DISPLAY;
    fn edit_ui(&mut self, ui: &mut Ui) -> Response {
        self.edit_ui_with_id(ui, "tuple")
    }

    fn edit_ui_with_id(&mut self, ui: &mut Ui, id: impl Hash) -> Response {
        let id = Id::new(id);
        let mut changed = false;
        let mut res = ui
            .group(|ui| {
                changed |= self.0.edit_ui_with_id(ui, id.with("first")).changed();
                changed |= self.1.edit_ui_with_id(ui, id.with("second")).changed();
            })
            .response;
        if changed {
            res.mark_changed();
        }
        res
    }
}
