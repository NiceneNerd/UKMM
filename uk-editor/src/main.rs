use roead::aamp::ParameterIO;
use roead::byml::Byml;
use roead::sarc::Sarc;
use roead::yaz0;
use uk_content::actor::residents::ResidentActors;
use uk_content::resource::{ASList, CookData};
use uk_ui::editor::EditableValue;
use uk_ui::egui;

struct EditorTest<T> {
    value: T,
}

impl<T: EditableValue> eframe::App for EditorTest<T> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([false, true]).show(ui, |ui| {
                self.value.edit_ui_with_id(ui, "aamp-test");
            });
        });
    }
}

fn main() {
    uk_ui::icons::load_icons();
    let actor = Sarc::new(
        yaz0::decompress(
            &std::fs::read("uk-content/test/Actor/Pack/Npc_TripMaster_00.sbactorpack").unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let data = ASList::try_from(
        &ParameterIO::from_binary(
            actor
                .get_file("Actor/ASList/Npc_TripMaster_00.baslist")
                .unwrap()
                .unwrap()
                .data,
        )
        .unwrap(),
    )
    .unwrap();
    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Box::new(EditorTest { value: data })),
    )
}
