use crate::{icons::IconButtonExt, syntect::CodeTheme};
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
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let id = Id::new(&self);
            let yaml = ui
                .memory()
                .data
                .get_temp_mut_or_insert_with(id, || Arc::new(RwLock::new(self.to_text().unwrap())))
                .clone();
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
            if ui.icon_button(super::icons::Icon::Check).clicked() {
                if let Ok(val) = roead::byml::Byml::from_text(yaml.read().as_str()) {
                    *self = val;
                }
            }
            egui::TextEdit::multiline(yaml.write().deref_mut())
                .layouter(&mut layouter)
                .code_editor()
                .desired_width(ui.available_width())
                .show(ui);
        })
        .response
    }
}
