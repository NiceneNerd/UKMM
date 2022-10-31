use roead::byml::Byml;
use std::ops::DerefMut;
use uk_ui::{
    editor::{EditableDisplay, EditableValue},
    egui,
    egui_extras::{self, Size},
    ext::UiExt,
    icons::IconButtonExt,
};

fn edit_flag_val(val: &mut Byml, ui: &mut egui::Ui, id: egui::Id) -> egui::Response {
    match val {
        Byml::String(v) => v.edit_ui_with_id(ui, id),
        Byml::BinaryData(v) => v.edit_ui_with_id(ui, id),
        Byml::Array(v) => v.edit_ui_with_id(ui, id),
        Byml::Hash(_) => unimplemented!(),
        Byml::Bool(v) => v.edit_ui_with_id(ui, id),
        Byml::I32(v) => v.edit_ui_with_id(ui, id),
        Byml::Float(v) => v.edit_ui_with_id(ui, id),
        Byml::U32(v) => v.edit_ui_with_id(ui, id),
        Byml::I64(v) => v.edit_ui_with_id(ui, id),
        Byml::U64(v) => v.edit_ui_with_id(ui, id),
        Byml::Double(v) => v.edit_ui_with_id(ui, id),
        Byml::Null => unimplemented!(),
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
        if let Some(ref name) = flag.data_name {
            ui.clipped_label(name.as_str());
        } else {
            ui.clipped_label(flag.hash_value.to_string());
        }
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
                    .column(Size::initial(text_height * 10.0))
                    .columns(Size::remainder(), 12)
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
                            self.flags.insert(
                                roead::aamp::hash_name(new_flag.read().as_str()),
                                super::FlagData::default(),
                            );
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
