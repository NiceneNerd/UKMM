use roead::aamp::ParameterIO;

use roead::byml::Byml;
use roead::sarc::Sarc;
use roead::yaz0;

use uk_content::data::gamedata::GameData;
use uk_content::resource::AIProgram;
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
    // let data = ResidentActors::try_from(
    //     &Byml::from_binary(&std::fs::read("uk-content/test/Actor/ResidentActors.byml").unwrap())
    //         .unwrap(),
    // )
    // .unwrap();
    // let actor = Sarc::new(
    //     yaz0::decompress(
    //         &std::fs::read("uk-content/test/Actor/Pack/Npc_TripMaster_00.sbactorpack").unwrap(),
    //     )
    //     .unwrap(),
    // )
    // .unwrap();
    // let data = AIProgram::try_from(
    //     &ParameterIO::from_binary(
    //         actor
    //             .get_file("Actor/AIProgram/NpcTripMaster.baiprog")
    //             .unwrap()
    //             .unwrap()
    //             .data,
    //     )
    //     .unwrap(),
    // )
    // .unwrap();
    fn load_gamedata_sarc() -> Sarc<'static> {
        Sarc::new(std::fs::read("uk-content/test/GameData/gamedata.ssarc").unwrap()).unwrap()
    }

    fn load_gamedata() -> Byml {
        let gs = load_gamedata_sarc();
        Byml::from_binary(gs.get_data("/bool_data_0.bgdata").unwrap().unwrap()).unwrap()
    }

    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| {
            Box::new(EditorTest {
                value: GameData::try_from(&load_gamedata()).unwrap(),
            })
        }),
    )
}
