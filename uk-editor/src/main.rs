use uk_content::actor::residents::ResidentActorData;
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
    let resident = ResidentActorData {
        only_res: true,
        scale: Some(
            [("x", 0.0.into()), ("y", 0.0.into()), ("z", 0.0.into())]
                .into_iter()
                .collect(),
        ),
    };
    eframe::run_native(
        "U-King Mod Editor",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Box::new(EditorTest { value: resident })),
    )
}
