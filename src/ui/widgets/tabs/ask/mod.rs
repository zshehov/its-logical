use self::growable_table::GrowableTable;

mod growable_table;

pub(crate) struct Ask {}

impl Ask {
    pub(crate) fn show(&self, ui: &mut egui::Ui) {
        ui.heading("Term name");
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            ui.vertical(|ui| {
                for i in 0..5 {
                    ui.label(format!("Arg numero {}", i));
                }
            });
            ui.separator();
            GrowableTable::new(5).show(ui);
        });
    }
}
