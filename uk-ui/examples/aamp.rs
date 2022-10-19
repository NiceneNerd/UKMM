use uk_ui::editor::EditableValue;

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
    let pio = roead::aamp::ParameterIO::from_binary(
        &std::fs::read("uk-content/test/Chemical/system.bchmres").unwrap(),
    )
    .unwrap();
    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Box::new(EditorTest { value: pio })),
    )
}
