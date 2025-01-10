use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};
use uk_ui::{
    egui::{self, Button, Key, Ui, Vec2},
    icons::{get_icon, Icon, IconButtonExt},
};

use super::{App, FocusedPane, Message, LOCALIZATION};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FilePickerState {
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub path_input: String,
    #[serde(skip)]
    pub selected: Option<PathBuf>,
    pub entries: Vec<PathBuf>,
}

// Tweak the macro-generated `Deserialize` impl to load file entries afresh
impl<'de> Deserialize<'de> for FilePickerState {
    fn deserialize<__D>(__deserializer: __D) -> serde::__private::Result<Self, __D::Error>
    where
        __D: serde::Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Field {
            __field0,
            __field1,
            __field2,
            __ignore,
        }
        #[doc(hidden)]
        struct __FieldVisitor;
        impl<'de> serde::de::Visitor<'de> for __FieldVisitor {
            type Value = __Field;

            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(__formatter, "field identifier")
            }

            fn visit_u64<__E>(self, __value: u64) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    0u64 => serde::__private::Ok(__Field::__field0),
                    1u64 => serde::__private::Ok(__Field::__field1),
                    2u64 => serde::__private::Ok(__Field::__field2),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }

            fn visit_str<__E>(self, __value: &str) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    "path" => serde::__private::Ok(__Field::__field0),
                    "history" => serde::__private::Ok(__Field::__field1),
                    "path_input" => serde::__private::Ok(__Field::__field2),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }

            fn visit_bytes<__E>(self, __value: &[u8]) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    b"path" => serde::__private::Ok(__Field::__field0),
                    b"history" => serde::__private::Ok(__Field::__field1),
                    b"path_input" => serde::__private::Ok(__Field::__field2),
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }
        }
        impl<'de> serde::Deserialize<'de> for __Field {
            #[inline]
            fn deserialize<__D>(__deserializer: __D) -> serde::__private::Result<Self, __D::Error>
            where
                __D: serde::Deserializer<'de>,
            {
                serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
            }
        }
        #[doc(hidden)]
        struct __Visitor<'de> {
            marker:   serde::__private::PhantomData<FilePickerState>,
            lifetime: serde::__private::PhantomData<&'de ()>,
        }
        impl<'de> serde::de::Visitor<'de> for __Visitor<'de> {
            type Value = FilePickerState;

            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(__formatter, "struct FilePickerState")
            }

            #[inline]
            fn visit_seq<__A>(
                self,
                mut __seq: __A,
            ) -> serde::__private::Result<Self::Value, __A::Error>
            where
                __A: serde::de::SeqAccess<'de>,
            {
                let __field0 = match serde::de::SeqAccess::next_element::<PathBuf>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            0usize,
                            &"struct FilePickerState with 3 elements",
                        ));
                    }
                };
                let __field1 = match serde::de::SeqAccess::next_element::<Vec<PathBuf>>(&mut __seq)?
                {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            1usize,
                            &"struct FilePickerState with 3 elements",
                        ));
                    }
                };
                let __field2 = match serde::de::SeqAccess::next_element::<String>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(serde::de::Error::invalid_length(
                            2usize,
                            &"struct FilePickerState with 3 elements",
                        ));
                    }
                };
                let __field3 = serde::__private::Default::default();
                let __field4 = FilePickerState::load_entries(&__field0);
                serde::__private::Ok(FilePickerState {
                    path: __field0,
                    history: __field1,
                    path_input: __field2,
                    selected: __field3,
                    entries: __field4,
                })
            }

            #[inline]
            fn visit_map<__A>(
                self,
                mut __map: __A,
            ) -> serde::__private::Result<Self::Value, __A::Error>
            where
                __A: serde::de::MapAccess<'de>,
            {
                let mut __field0: serde::__private::Option<PathBuf> = serde::__private::None;
                let mut __field1: serde::__private::Option<Vec<PathBuf>> = serde::__private::None;
                let mut __field2: serde::__private::Option<String> = serde::__private::None;
                while let serde::__private::Some(__key) =
                    serde::de::MapAccess::next_key::<__Field>(&mut __map)?
                {
                    match __key {
                        __Field::__field0 => {
                            if serde::__private::Option::is_some(&__field0) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field("path"),
                                );
                            }
                            __field0 =
                                serde::__private::Some(
                                    serde::de::MapAccess::next_value::<PathBuf>(&mut __map)?,
                                );
                        }
                        __Field::__field1 => {
                            if serde::__private::Option::is_some(&__field1) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field("history"),
                                );
                            }
                            __field1 = serde::__private::Some(serde::de::MapAccess::next_value::<
                                Vec<PathBuf>,
                            >(
                                &mut __map
                            )?);
                        }
                        __Field::__field2 => {
                            if serde::__private::Option::is_some(&__field2) {
                                return serde::__private::Err(
                                    <__A::Error as serde::de::Error>::duplicate_field("path_input"),
                                );
                            }
                            __field2 =
                                serde::__private::Some(serde::de::MapAccess::next_value::<String>(
                                    &mut __map,
                                )?);
                        }
                        _ => {
                            let _ = serde::de::MapAccess::next_value::<serde::de::IgnoredAny>(
                                &mut __map,
                            )?;
                        }
                    }
                }
                let __field0 = match __field0 {
                    serde::__private::Some(__field0) => __field0,
                    serde::__private::None => serde::__private::de::missing_field("path")?,
                };
                let __field1 = match __field1 {
                    serde::__private::Some(__field1) => __field1,
                    serde::__private::None => serde::__private::de::missing_field("history")?,
                };
                let __field2 = match __field2 {
                    serde::__private::Some(__field2) => __field2,
                    serde::__private::None => serde::__private::de::missing_field("path_input")?,
                };
                serde::__private::Ok(FilePickerState {
                    entries: FilePickerState::load_entries(&__field0),
                    path: __field0,
                    history: __field1,
                    path_input: __field2,
                    selected: serde::__private::Default::default(),
                })
            }
        }
        #[doc(hidden)]
        const FIELDS: &[&str] = &["path", "history", "path_input"];
        serde::Deserializer::deserialize_struct(
            __deserializer,
            "FilePickerState",
            FIELDS,
            __Visitor {
                marker:   serde::__private::PhantomData::<FilePickerState>,
                lifetime: serde::__private::PhantomData,
            },
        )
    }
}

impl Default for FilePickerState {
    fn default() -> Self {
        let path = dirs2::download_dir().or_else(dirs2::home_dir).unwrap();
        Self {
            path_input: path.display().to_string(),
            entries: Self::load_entries(&path),
            path,
            history: vec![],
            selected: None,
        }
    }
}

impl FilePickerState {
    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        self.path_input = path.display().to_string();
        self.entries = Self::load_entries(&path);
        self.path = path;
    }

    fn load_entries(path: &Path) -> Vec<PathBuf> {
        if let Ok(dir_entries) =
            fs::read_dir(path).map(|entries| entries.filter_map(std::result::Result::ok))
        {
            let mut entries = dir_entries
                .filter_map(|e| {
                    let path = e.path();
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    ((matches!(ext.as_str(), "zip" | "7z" | "bnp") || path.is_dir())
                        && !e.file_name().to_str().unwrap_or("").starts_with('.'))
                    .then_some(path)
                })
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| {
                if a.is_file() != b.is_file() {
                    b.is_dir().cmp(&a.is_dir())
                } else {
                    a.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(
                            &b.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_lowercase(),
                        )
                }
            });
            entries
        } else {
            Default::default()
        }
    }
}

impl App {
    pub fn render_file_picker(&mut self, ui: &mut Ui) {
        let loc = LOCALIZATION.read();
        egui::Frame::none().inner_margin(2.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (icon, tooltip, cb) in [
                    (
                        Icon::FolderOpen,
                        loc.get("Menu_File_Open"),
                        Box::new(|| self.do_update(Message::SelectFile)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::ArrowUp,
                        loc.get("FilePicker_Up"),
                        Box::new(|| self.do_update(Message::FilePickerUp)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::ArrowBack,
                        loc.get("FilePicker_Back"),
                        Box::new(|| self.do_update(Message::FilePickerBack)) as Box<dyn FnOnce()>,
                    ),
                    (
                        Icon::Refresh,
                        loc.get("FilePicker_Refresh"),
                        Box::new(|| {
                            self.do_update(Message::FilePickerSet(Some(
                                self.picker_state.path.clone(),
                            )))
                        }) as Box<dyn FnOnce()>,
                    ),
                ] {
                    if ui.icon_button(icon).on_hover_text(tooltip).clicked() {
                        cb();
                    }
                }
                let res = ui.text_edit_singleline(&mut self.picker_state.path_input);
                if res.changed() {
                    self.do_update(Message::FilePickerSet(None));
                }
            });
            egui::ScrollArea::both()
                .id_source("file_picker")
                .show(ui, |ui| {
                    ui.add_space(8.);
                    ui.style_mut().spacing.item_spacing.y = 4.;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke.width = 0.0;
                    let entries = &self.picker_state.entries;
                    if self.focused == FocusedPane::FilePicker && !self.modal_open() {
                        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            let pos = match entries
                                .iter()
                                .position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                Some(p) => (p + 1).min(entries.len() - 1),
                                None => 0,
                            };
                            self.picker_state.selected = Some(entries[pos].to_path_buf());
                        } else if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            let pos = match entries
                                .iter()
                                .position(|p| self.picker_state.selected.as_ref() == Some(p))
                            {
                                Some(p) => p.max(1) - 1,
                                None => 0,
                            };
                            self.picker_state.selected = Some(entries[pos].to_path_buf());
                        }
                    }
                    entries.clone().iter().for_each(|path| {
                        self.render_picker_dir_entry(path, ui);
                    });
                    ui.allocate_space(ui.available_size());
                });
        });
    }

    fn render_picker_dir_entry(&mut self, path: &Path, ui: &mut Ui) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let is_dir = path.is_dir();
        let selected = self.picker_state.selected.as_ref().map(|p| p.as_ref()) == Some(path);
        let _icon_size: Vec2 = [ui.spacing().icon_width, ui.spacing().icon_width].into();
        let res = ui.add(
            Button::image_and_text(
                get_icon(
                    ui.ctx(),
                    if is_dir {
                        Icon::Folder
                    } else {
                        Icon::FolderZip
                    },
                ),
                name,
            )
            .wrap_mode(egui::TextWrapMode::Truncate)
            // .tint(if is_dir {
            //     visuals::YELLOW
            // } else {
            //     visuals::GREEN
            // })
            .fill(if selected {
                ui.style().visuals.selection.bg_fill
            } else {
                ui.style().visuals.noninteractive().bg_fill
            }),
        );
        if res.double_clicked()
            || (ui.input(|i| i.key_pressed(Key::Enter)) && selected && !self.modal_open())
        {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            if path.is_dir() {
                self.do_update(Message::FilePickerSet(Some(path.to_path_buf())));
            } else {
                self.do_update(Message::OpenMod(path.to_path_buf()));
            }
        } else if res.clicked() {
            self.do_update(Message::SetFocus(FocusedPane::FilePicker));
            self.picker_state.selected = Some(path.to_path_buf());
        }
    }
}
