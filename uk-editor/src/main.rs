use roead::byml::Byml;
use uk_content::actor::residents::ResidentActors;
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
    let residents = ResidentActors::try_from(
        &Byml::from_binary(&std::fs::read("uk-content/test/Actor/ResidentActors.byml").unwrap())
            .unwrap(),
    )
    .unwrap();
    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Box::new(EditorTest { value: residents })),
    )
}
