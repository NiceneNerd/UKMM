use egui::{text::LayoutJob, Align, Button, Rect, Response, TextFormat, TextStyle, Ui};
use material_icons::Icon;
pub trait IconButtonExt {
    fn icon_button(&mut self, icon: Icon) -> Response;
}

impl IconButtonExt for Ui {
    fn icon_button(&mut self, icon: Icon) -> Response {
        let text: char = icon.into();
        let fmt = TextFormat {
            valign: Align::Max,
            ..Default::default()
        };
        let job = LayoutJob::single_section(
            text.into(),
            TextFormat {
                valign: Align::Center,
                ..Default::default()
            },
        );
        let mut gallery = self.fonts().layout_job(job);
        std::sync::Arc::make_mut(&mut gallery).rect.min.y = 8.;
        self.scope(|ui| ui.button(gallery)).response
    }
}
