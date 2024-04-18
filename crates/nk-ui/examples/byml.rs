use nk_ui::editor::EditableValue;

struct EditorTest<T> {
    value: T,
}

impl<T: EditableValue> eframe::App for EditorTest<T> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([false, true]).show(ui, |ui| {
                self.value.edit_ui_with_id(ui, "byml-test");
            });
        });
    }
}

fn main() {
    nk_ui::icons::load_icons();
    let byml = roead::byml::Byml::from_binary(
        std::fs::read("uk-content/test/Actor/ResidentActors.byml").unwrap(),
    )
    .unwrap();
    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::new(EditorTest { value: byml })),
    )
}
