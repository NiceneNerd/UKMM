use std::hash::{Hash, Hasher};

use egui::{DragValue, InnerResponse, Response, Ui, WidgetText};

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
        let mut res = ui.vertical(|ui| {
            self.iter_mut().for_each(|v| {
                changed = changed || v.edit_ui(ui).changed();
            });
        });
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
            roead::byml::Byml::Hash(v) => todo!(),
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

pub fn editable_map_fixed_keys<'a>(
    iter: impl Iterator<Item = (impl AsRef<str> + 'a, &'a mut (impl EditableValue + 'a))>,
    id_source: impl Hash,
    ui: &mut Ui,
) -> Response {
    let mut changed = false;
    let mut res = egui::Grid::new(id_source)
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            iter.for_each(|(k, v)| {
                ui.label(k.as_ref());
                changed = changed || v.edit_ui(ui).changed();
                ui.end_row();
            });
        });
    if changed {
        res.response.mark_changed();
    }
    res.response
}

pub fn editable_map<'a>(
    iter: impl Iterator<Item = &'a mut (impl EditableValue + 'a, impl EditableValue + 'a)>,
    id_source: impl Hash,
    ui: &mut Ui,
) -> Response {
    let mut changed = false;
    let mut res = egui::Grid::new(id_source)
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            iter.for_each(|(k, v)| {
                changed = changed || k.edit_ui(ui).changed();
                changed = changed || v.edit_ui(ui).changed();
                ui.end_row();
            });
        });
    if changed {
        res.response.mark_changed();
    }
    res.response
}
