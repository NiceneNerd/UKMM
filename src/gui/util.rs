use std::any::TypeId;

pub struct SmartStringWrapper<'a>(pub &'a mut smartstring::alias::String);

impl uk_ui::egui::TextBuffer for SmartStringWrapper<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    #[inline]
    fn as_str(&self) -> &str {
        self.0.as_str()
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

    #[inline]
    fn type_id(&self) -> TypeId {
        TypeId::of::<smartstring::alias::String>()
    }
}
