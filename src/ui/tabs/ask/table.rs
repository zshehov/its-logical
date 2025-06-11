use egui::RichText;

pub(crate) struct Table {
    content: Vec<Vec<String>>,
}

impl Table {
    pub fn set_content(&mut self, content: Vec<Vec<String>>) {
        self.content = content;
    }

    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }
}

impl Table {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> bool {
        egui::ScrollArea::horizontal()
            .show(ui, |ui| {
                for item in &self.content {
                    ui.vertical(|ui| {
                        for i in item {
                            ui.add(egui::Label::new(RichText::new(i).monospace()));
                        }
                    });
                }
                ui.add(egui::Button::new(RichText::new("Consult").heading()))
                    .clicked()
            })
            .inner
    }
}
