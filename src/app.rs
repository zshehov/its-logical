pub struct ItsLogicalApp {
    ask_tab: AskTab,
    term_tabs: Vec<TermTab>,
    current_tab: TabKind,
    terms: Vec<Term>,
}

struct Term {
    name: String,
}

struct AskTab {}
impl Tab for AskTab {
    fn name(&self) -> String {
        "Ask".to_owned()
    }

    fn show(&self, ui: &mut egui::Ui) {
        ui.heading("Ask a question");
        ui.separator();
    }
}

struct TermTab {
    name: String,
}

impl Tab for TermTab {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn show(&self, ui: &mut egui::Ui) {
        ui.heading(&self.name);
        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("description_scroll_area")
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
                    |ui| {
                        ui.label(
                            egui::RichText::new("A mother is a parent that is female").italics(),
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
                        let mut first_param = String::new();
                        let mut second_param = String::new();
                        ui.horizontal(|ui| {
                            create_rule_placeholder(
                                ui,
                                "mother",
                                &mut [&mut first_param, &mut second_param],
                            );
                            ui.small_button("+");
                        });
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
                        let mut first_param = String::new();
                        let mut second_param = String::new();
                        ui.horizontal(|ui| {
                            create_placeholder(
                                ui,
                                "mother",
                                &mut [&mut first_param, &mut second_param],
                            );
                            ui.small_button("+");
                        });
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
    }
}

#[derive(PartialEq)]
enum TabKind {
    Ask,
    Term(usize),
}

trait Tab {
    fn name(&self) -> String;
    fn show(&self, ui: &mut egui::Ui);
}

impl Default for ItsLogicalApp {
    fn default() -> Self {
        return Self {
            ask_tab: AskTab {},
            term_tabs: vec![],
            current_tab: TabKind::Ask,
            terms: vec![
                Term {
                    name: "mother".to_owned(),
                },
                Term {
                    name: "father".to_owned(),
                },
            ],
        };
    }
}

impl ItsLogicalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Default::default()
    }
}

impl eframe::App for ItsLogicalApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let scroll_area = egui::ScrollArea::vertical().auto_shrink([false; 2]);
            scroll_area.show(ui, |ui| {
                for term in &self.terms {
                    if ui.small_button(&term.name).clicked() {
                        self.term_tabs.push(TermTab {
                            name: term.name.to_owned(),
                        })
                    }
                }
            })
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.current_tab,
                    TabKind::Ask,
                    egui::RichText::new(self.ask_tab.name()).strong(),
                );
                ui.separator();

                for (i, tab) in self.term_tabs.iter().enumerate() {
                    ui.selectable_value(&mut self.current_tab, TabKind::Term(i), tab.name());
                }
            });
            ui.separator();

            match self.current_tab {
                TabKind::Term(idx) => {
                    self.term_tabs.get(idx).unwrap().show(ui);
                }
                TabKind::Ask => self.ask_tab.show(ui),
            }
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

// expects to be called in a horizontal layout
fn create_placeholder(ui: &mut egui::Ui, term_name: &str, parameters: &mut [&mut String]) {
    ui.label(egui::RichText::new(format!("{}(", term_name)).weak());

    let mut added_once = false;
    for param in parameters {
        if added_once {
            ui.label(egui::RichText::new(",").weak());
        }
        ui.add(egui::TextEdit::singleline(*param).hint_text("X"));
        added_once = true
    }
    ui.label(egui::RichText::new(")").weak());
}

// expects to be called in a horizontal layout
fn create_rule_placeholder(ui: &mut egui::Ui, term_name: &str, parameters: &mut [&mut String]) {
    create_placeholder(ui, term_name, parameters);
    ui.label(egui::RichText::new(" if ").weak());

    // TODO: pass this from the outside
    let mut rule_string = "";
    ui.add(egui::TextEdit::singleline(&mut rule_string).hint_text("ruuuule"));
}
