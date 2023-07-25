use egui::{Color32, RichText, TextStyle};

pub(crate) struct GrowableTable {
    fixed_dimension: usize,
    contents: Vec<Vec<String>>,
    current_growable_size: f32,
}

impl GrowableTable {
    pub(crate) fn new(fixed_dimension: usize) -> Self {
        Self {
            fixed_dimension,
            contents: vec![],
            current_growable_size: 0.0,
        }
    }
    fn grow(&mut self, ui: &mut egui::Ui, new_content: &[String]) {
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
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        for _ in 0..100 {
            self.grow(
                ui,
                &[
                    "first".to_string(),
                    "second".to_string(),
                    "third".to_string(),
                    "fourth".to_string(),
                    "fifth".to_string(),
                ],
            );
        }
        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.set_min_width(self.current_growable_size);
                for item in &self.contents {
                    ui.vertical(|ui| {
                        for i in item {
                            ui.label(RichText::new(i).monospace());
                        }
                    });
                }
            })
        });
    }
}
