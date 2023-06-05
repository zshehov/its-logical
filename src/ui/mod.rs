use egui::Context;

use crate::term_knowledge_base::TermsKnowledgeBase;

use self::widgets::tabs::{ChosenTab, Tabs};

mod change_propagator;
mod widgets;

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: Tabs,
    terms: T,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            term_tabs: Tabs::default(),
            terms,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            if ui
                .button(egui::RichText::new("Add term").underline().strong())
                .clicked()
            {
                self.term_tabs.add_new_term();
            };
            let term_list_selection = widgets::terms_list::show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.select_with_push(&term_name, &self.terms);
            }
        });

        let chosen_tab = egui::TopBottomPanel::top("tabs_panel")
            .show(ctx, |ui| {
                return self.term_tabs.show(ui);
            })
            .inner;

        match chosen_tab {
            ChosenTab::Term(term_screen) => {
                let changes = egui::CentralPanel::default()
                    .show(ctx, |ui| term_screen.show(ui, &mut self.terms))
                    .inner;

                if let Some(changes) = changes {
                    let (all_changes, needs_confirmation) =
                        change_propagator::apply_changes(changes, &self.terms);

                    if needs_confirmation {
                        for (_, updated_term) in all_changes {
                            self.term_tabs.force_open_in_edit(&updated_term);
                        }
                    } else {
                        for (term_name, updated_term) in all_changes {
                            self.terms.put(&term_name, updated_term).unwrap();
                            self.term_tabs.force_reload(&term_name, &self.terms);
                        }
                    }
                }
            }
            ChosenTab::Ask(_) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    widgets::ask::show(ui);
                });
            }
        };
    }
}
