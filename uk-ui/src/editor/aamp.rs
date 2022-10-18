use super::EditableValue;
use roead::{
    aamp::Parameter,
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
impl_edit_veclike!(Color, r, g, b, a);
impl_edit_veclike!(Quat, a, b, c, d);

struct FixedSafeStringWrapper<'a, const N: usize>(&'a mut roead::types::FixedSafeString<N>);

impl<const N: usize> egui::TextBuffer for FixedSafeStringWrapper<'_, N> {
    #[inline]
    fn as_str(&self) -> &str {
        self.as_str()
    }

    fn is_mutable(&self) -> bool {
        true
    }

    #[inline]
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let index = self.byte_index_from_char_index(char_index);
        let end = (index + text.len()).min(N);
        todo!();
        text.len()
    }

    #[inline]
    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        assert!(char_range.start <= char_range.end);
        let start = self.byte_index_from_char_index(char_range.start);
        let end = self.byte_index_from_char_index(char_range.end);
        todo!()
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
            Parameter::String32(_) => todo!(),
            Parameter::String64(_) => todo!(),
            Parameter::Curve1(_) => todo!(),
            Parameter::Curve2(_) => todo!(),
            Parameter::Curve3(_) => todo!(),
            Parameter::Curve4(_) => todo!(),
            Parameter::BufferInt(_) => todo!(),
            Parameter::BufferF32(_) => todo!(),
            Parameter::String256(_) => todo!(),
            Parameter::Quat(v) => v.edit_ui(ui),
            Parameter::U32(v) => v.edit_ui(ui),
            Parameter::BufferU32(_) => todo!(),
            Parameter::BufferBinary(v) => v.edit_ui(ui),
            Parameter::StringRef(v) => v.edit_ui(ui),
        }
    }
}
