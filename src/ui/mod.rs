use egui::Context;

use crate::model::term::Term;

mod widgets;

pub struct App {
    term_tabs: Vec<widgets::tabs::Tab>,
    current_tab: widgets::tabs::Tab,
    terms: Vec<Term>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            term_tabs: vec![],
            current_tab: widgets::tabs::Tab {
                name: "Ask".to_owned(),
                kind: widgets::tabs::TabKind::Ask,
            },
            terms: vec![
                Term::new(
                    "mother",
                    "a mother is a parent that's female",
                    &["MotherName", "ChildName"],
                    vec![
                        vec![Some("Siika".to_owned()), Some("Mircho".to_owned())],
                        vec![Some("Stefka".to_owned()), Some("Petko".to_owned())],
                    ],
                    vec![(
                        vec![Some("X".to_owned()), Some("Y".to_owned())],
                        "parent(X, Y) and female(X)".to_owned(),
                    )],
                ),
                Term::new(
                    "father",
                    "a father is a parent that's male",
                    &["FatherName", "ChildName"],
                    vec![
                        vec![Some("Krustio".to_owned()), Some("Mircho".to_owned())],
                        vec![Some("Stefcho".to_owned()), Some("Mitko".to_owned())],
                    ],
                    vec![(
                        vec![Some("X".to_owned()), Some("Y".to_owned())],
                        "parent(X, Y) and male(X)".to_owned(),
                    )],
                ),
                Term::new(
                    "male",
                    "male is one of the 2 genders",
                    &["PersonName"],
                    vec![
                        vec![Some("Krustio".to_owned())],
                        vec![Some("Mircho".to_owned())],
                        vec![Some("Stefcho".to_owned())],
                        vec![Some("Mitko".to_owned())],
                    ],
                    vec![(
                        vec![Some("PersonName".to_owned())],
                        "chromosomes(PersonName, Chromosomes) and Chromosomes == [X,Y]".to_owned(),
                    )],
                ),
            ],
        }
    }
}

impl App {
    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let term_list_selection = widgets::terms_list::show(ui, self.terms.iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.push(widgets::tabs::Tab {
                    name: term_name,
                    kind: widgets::tabs::TabKind::Term(self.term_tabs.len()),
                })
            }
        });
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            widgets::tabs::show(ui, &mut self.current_tab, self.term_tabs.iter().cloned());
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab.kind {
                widgets::tabs::TabKind::Ask => widgets::ask::show(ui),
                widgets::tabs::TabKind::Term(idx) => {
                    widgets::term::show(ui, self.terms.get(idx).unwrap());
                }
            }
        });
    }
}
