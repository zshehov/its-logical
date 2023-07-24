use egui::Context;
use tracing::debug;

use crate::{model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase};

use self::widgets::{tabs::Tabs, terms_list::TermList};

mod changes_handling;
mod widgets;

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: Tabs,
    term_list: TermList,
    terms: T,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            term_tabs: Tabs::default(),
            term_list: TermList::new(),
            terms,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            if let Some(output) = self.term_list.show(ui, self.terms.keys().iter()) {
                match output {
                    widgets::terms_list::TermListOutput::AddTerm(new_term_name) => {
                        let new_term = FatTerm::default();
                        if self.term_tabs.select(&new_term.meta.term.name) {
                            debug!("unfinished term creation present");
                            return;
                        }
                        self.term_tabs.push(&new_term);
                        self.term_tabs.select(&new_term.meta.term.name);

                        let new_term_screen = self
                            .term_tabs
                            .term_tabs
                            .get_mut(&new_term.meta.term.name)
                            .expect("the newly created term was just added");

                        new_term_screen.start_changes();
                        let (_, editing) = new_term_screen.get_pits_mut();
                        editing.expect("a").set_name(&new_term_name);
                    }
                    widgets::terms_list::TermListOutput::SelectedTerm(selected_term) => {
                        if !self.term_tabs.select(&selected_term) {
                            self.term_tabs
                                .push(&self.terms.get(&selected_term).unwrap());
                            self.term_tabs.select(&selected_term);
                        }
                    }
                }
            }
        });

        self.term_tabs.show(ctx, &mut self.terms)
    }
}
