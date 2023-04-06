/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ItsLogicalApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: f32,
}

impl Default for ItsLogicalApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl ItsLogicalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for ItsLogicalApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self { label, value } = self;

        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);
            scroll_area.show(ui, |ui| {
                // TODO: Add actual terms from the DB
                for item in 1..=50 {
                    ui.label(format!("this is item: {}", item));
                }
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Mother");
            ui.separator();

            egui::ScrollArea::vertical()
                .id_source("description_scroll_area")
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            ui.label(
                                egui::RichText::new("A mother is a parent that is female")
                                    .italics()
                                    .small(),
                            )
                        },
                    )
                });
            ui.separator();
            // Rules:
            egui::ScrollArea::vertical()
                .id_source("rules_scroll_area")
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            ui.label("mother(X,Y) if parent(X,Y) and female(X)");
                            ui.label("mother(_,_) if _________________________ +");
                        },
                    )
                });
            ui.separator();
            // Facts:
            egui::ScrollArea::vertical()
                .id_source("facts_scroll_area")
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            ui.label("mother(amy,steve)");
                            ui.label("mother(kunka,mitko)");
                            ui.label("mother(_,_) +");
                        },
                    )
                });
            ui.separator();
            // Reffered by:
            egui::ScrollArea::vertical()
                .id_source("referred_by_scroll_area")
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                        |ui| {
                            ui.label("Referred by:");
                            ui.label("grandmother");
                        },
                    )
                });
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally choose either panels OR windows.");
            });
        }
    }
}
