use std::{
    ops::DerefMut,
    sync::{atomic::AtomicUsize, Arc},
};

use lighter::lighter;
use roead::byml::Byml;
use uk_ui::{
    editor::{EditableDisplay, EditableValue},
    egui,
    egui_extras::{self, Column},
    ext::UiExt,
    icons::IconButtonExt,
};

fn edit_flag_val(val: &mut Byml, ui: &mut egui::Ui, id: egui::Id) -> egui::Response {
    match val {
        Byml::String(v) => v.edit_ui_with_id(ui, id),
        Byml::BinaryData(v) => v.edit_ui_with_id(ui, id),
        Byml::Array(v) => v.edit_ui_with_id(ui, id),
        Byml::Map(_) => unimplemented!(),
        Byml::Bool(v) => v.edit_ui_with_id(ui, id),
        Byml::I32(v) => v.edit_ui_with_id(ui, id),
        Byml::Float(v) => v.edit_ui_with_id(ui, id),
        Byml::U32(v) => v.edit_ui_with_id(ui, id),
        Byml::I64(v) => v.edit_ui_with_id(ui, id),
        Byml::U64(v) => v.edit_ui_with_id(ui, id),
        Byml::Double(v) => v.edit_ui_with_id(ui, id),
        _ => unimplemented!(),
    }
}

fn edit_flag_ui(
    flag: &mut (super::FlagData, bool),
    row: &mut egui_extras::TableRow,
    id: egui::Id,
) -> bool {
    let (flag, del) = flag;
    let mut changed = false;
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        ui.clipped_label(flag.data_name.as_str());
        // if let Some(ref name) = flag.data_name {
        //     ui.clipped_label(name.as_str());
        // } else {
        //     ui.clipped_label(flag.hash_value.to_string());
        // }
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .delete_rev
                .edit_ui_with_id(ui, id.with("delete_rev"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed |= edit_flag_val(&mut flag.init_value, ui, id.with("init_value")).changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .is_event_associated
                .edit_ui_with_id(ui, id.with("is_event_associated"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .is_one_trigger
                .edit_ui_with_id(ui, id.with("is_one_trigger"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .is_program_readable
                .edit_ui_with_id(ui, id.with("is_program_readable"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .is_program_writable
                .edit_ui_with_id(ui, id.with("is_program_writable"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .is_save
                .edit_ui_with_id(ui, id.with("is_save"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed |= edit_flag_val(&mut flag.max_value, ui, id.with("max_value")).changed()
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed |= edit_flag_val(&mut flag.min_value, ui, id.with("min_value")).changed()
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed = changed
            || flag
                .reset_type
                .edit_ui_with_id(ui, id.with("reset_type"))
                .changed();
    });
    row.col(|ui| {
        if *del {
            ui.visuals_mut().override_text_color = Some(uk_ui::visuals::RED);
        }
        changed |= del.edit_ui_with_id(ui, id.with("delete")).changed();
    });
    changed
}

impl EditableValue for super::GameData {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "game_data")
    }

    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let base_height: f32 = ui.spacing().interact_size.y;
        let text_height = ui.text_style_height(&egui::TextStyle::Body);
        let mut inner_id = egui::Id::new("");
        let res = egui::CollapsingHeader::new("GameData")
            .id_source(id)
            .show(ui, |ui| {
                inner_id = ui.id();
                egui_extras::TableBuilder::new(ui)
                    .resizable(true)
                    .column(Column::initial(text_height * 10.0))
                    .columns(Column::remainder(), 12)
                    .header(text_height, |mut header| {
                        header.col(|ui| {
                            ui.clipped_label("Flag");
                        });
                        header.col(|ui| {
                            ui.clipped_label("DeleteRev");
                        });
                        header.col(|ui| {
                            ui.clipped_label("InitValue");
                        });
                        header.col(|ui| {
                            ui.clipped_label("IsEventAssociated");
                        });
                        header.col(|ui| {
                            ui.clipped_label("IsOneTrigger");
                        });
                        header.col(|ui| {
                            ui.clipped_label("IsProgramReadable");
                        });
                        header.col(|ui| {
                            ui.clipped_label("IsProgramWritable");
                        });
                        header.col(|ui| {
                            ui.clipped_label("IsSave");
                        });
                        header.col(|ui| {
                            ui.clipped_label("MaxValue");
                        });
                        header.col(|ui| {
                            ui.clipped_label("MinValue");
                        });
                        header.col(|ui| {
                            ui.clipped_label("ResetType");
                        });
                        header.col(|ui| {
                            ui.label("Delete");
                        });
                    })
                    .body(|body| {
                        body.rows(base_height, self.flags.len(), |i, mut row| {
                            if let Some((hash, flag)) = self.flags.iter_full_mut().nth(i) {
                                changed |= edit_flag_ui(flag, &mut row, id.with(hash));
                            }
                        });
                    });
                let mut clear_flag = false;
                let new_flag_id = id.with("new_flag");
                if let Some(new_flag) = ui.get_temp_string(new_flag_id) {
                    ui.horizontal(|ui| {
                        changed = changed
                            || ui
                                .text_edit_singleline(new_flag.write().deref_mut())
                                .changed();
                        if ui.icon_button(uk_ui::icons::Icon::Check).clicked() {
                            self.flags
                                .insert(new_flag.read().as_str(), super::FlagData::default());
                            clear_flag = true;
                        }
                    });
                }
                if ui.icon_button(uk_ui::icons::Icon::Add).clicked() {
                    ui.create_temp_string(new_flag_id, None);
                }
                if clear_flag {
                    ui.clear_temp_string(new_flag_id);
                }
            });
        if res.header_response.changed() {
            let table_id = inner_id.with("__table_resize");
            ui.data().remove::<Vec<f32>>(table_id);
        }
        let mut res = res.body_response.unwrap_or(res.header_response);
        if changed {
            res.mark_changed();
        }
        res
    }
}

static DATA_TYPES: &[&str] = &[
    "bool_array_data",
    "bool_data",
    "f32_array_data",
    "f32_data",
    "revival_bool_data",
    "revival_s32_data",
    "s32_array_data",
    "s32_data",
    "string32_data",
    "string64_array_data",
    "string64_data",
    "string256_array_data",
    "string256_data",
    "vector2f_array_data",
    "vector2f_data",
    "vector3f_array_data",
    "vector3f_data",
    "vector4f_data",
];

impl EditableValue for super::GameDataPack {
    const DISPLAY: EditableDisplay = EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        self.edit_ui_with_id(ui, "game_data_pack")
    }

    #[allow(clippy::needless_borrow)]
    fn edit_ui_with_id(&mut self, ui: &mut egui::Ui, id: impl std::hash::Hash) -> egui::Response {
        let id = egui::Id::new(id);
        let mut changed = false;
        let selected = ui
            .data()
            .get_temp_mut_or_default::<Arc<AtomicUsize>>(id.with("current"))
            .clone();
        let res = egui::CollapsingHeader::new("GameDataPack")
            .id_source(id)
            .show(ui, |ui| {
                let mut select = selected.load(std::sync::atomic::Ordering::Relaxed);
                if egui::ComboBox::new(id.with("combo"), "Data Type")
                    .show_index(ui, &mut select, DATA_TYPES.len(), |i| {
                        DATA_TYPES[i].to_owned()
                    })
                    .changed()
                {
                    selected.store(select, std::sync::atomic::Ordering::Relaxed);
                }
                changed |= lighter! {
                    match DATA_TYPES[selected.load(std::sync::atomic::Ordering::Relaxed)] {
                        "bool_array_data" => self.bool_array_data.edit_ui_with_id(ui, id.with("inner")),
                        "bool_data" => self.bool_data.edit_ui_with_id(ui, id.with("inner")),
                        "f32_array_data" => self.f32_array_data.edit_ui_with_id(ui, id.with("inner")),
                        "f32_data" => self.f32_data.edit_ui_with_id(ui, id.with("inner")),
                        "revival_bool_data" => {
                            self.revival_bool_data.edit_ui_with_id(ui, id.with("inner"))
                        }
                        "revival_s32_data" => {
                            self.revival_s32_data.edit_ui_with_id(ui, id.with("inner"))
                        }
                        "s32_array_data" => self.s32_array_data.edit_ui_with_id(ui, id.with("inner")),
                        "s32_data" => self.s32_data.edit_ui_with_id(ui, id.with("inner")),
                        "string32_data" => self.string32_data.edit_ui_with_id(ui, id.with("inner")),
                        "string64_array_data" => {
                            self.string64_array_data
                                .edit_ui_with_id(ui, id.with("inner"))
                        }
                        "string64_data" => self.string64_data.edit_ui_with_id(ui, id.with("inner")),
                        "string256_array_data" => {
                            self.string256_array_data
                                .edit_ui_with_id(ui, id.with("inner"))
                        }
                        "string256_data" => self.string256_data.edit_ui_with_id(ui, id.with("inner")),
                        "vector2f_array_data" => {
                            self.vector2f_array_data
                                .edit_ui_with_id(ui, id.with("inner"))
                        }
                        "vector2f_data" => self.vector2f_data.edit_ui_with_id(ui, id.with("inner")),
                        "vector3f_array_data" => {
                            self.vector3f_array_data
                                .edit_ui_with_id(ui, id.with("inner"))
                        }
                        "vector3f_data" => self.vector3f_data.edit_ui_with_id(ui, id.with("inner")),
                        "vector4f_data" => self.vector4f_data.edit_ui_with_id(ui, id.with("inner")),
                        _ => unsafe { std::hint::unreachable_unchecked() },
                    }
                }
                .changed();
            });
        let mut res = res.body_response.unwrap_or(res.header_response);
        if changed {
            res.mark_changed();
        }
        res
    }
}
