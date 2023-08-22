use egui::{Color32, RichText, TextStyle};

pub(crate) struct GrowableTable {
    contents: Vec<Vec<String>>,
    current_growable_size: f32,
}

impl GrowableTable {
    pub(crate) fn new() -> Self {
        Self {
            contents: vec![],
            current_growable_size: 0.0,
        }
    }
    pub(crate) fn grow(&mut self, ui: &mut egui::Ui, new_content: &[String]) {
        self.contents.push(new_content.to_vec());

        let monospace_style = TextStyle::resolve(&TextStyle::Monospace, ui.style());

        let (single_char_width, mid_chars_width) = ui.fonts(|f| {
            let single_char_width = f.glyph_width(&monospace_style, 'c');
            let mid_chars_width = f
                .layout_no_wrap("cc".to_string(), monospace_style.clone(), Color32::WHITE)
                .rect
                .max
                .x
                - 2.0 * single_char_width;
            (single_char_width, mid_chars_width)
        });

        let longset_string = new_content
            .iter()
            .max_by(|x, y| x.len().cmp(&y.len()))
            .map(String::len)
            .unwrap_or(0);

        self.current_growable_size += ui.spacing().item_spacing.x;
        self.current_growable_size += (longset_string as f32) * single_char_width
            + (longset_string - 1) as f32 * mid_chars_width;
    }
}

impl GrowableTable {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> bool {
        egui::ScrollArea::horizontal()
            .show(ui, |ui| {
                for item in &self.contents {
                    ui.vertical(|ui| {
                        for i in item {
                            ui.add(egui::Label::new(RichText::new(i).monospace()).wrap(false));
                        }
                    });
                }
                ui.add(egui::Button::new(RichText::new("More ...").heading()).wrap(false))
                    .clicked()
            })
            .inner
    }
}
