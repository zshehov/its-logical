use egui::Context;

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
            ui.heading("Terms");
            ui.separator();

            if ui
                .button(egui::RichText::new("Add term").underline().strong())
                .clicked()
            {
                let new_term = FatTerm::default();
                // TODO: maybe this will break if multiple new tabs are opened - maybe use rev
                // iterator?
                self.term_tabs.push(&new_term);
                self.term_tabs.select(&new_term.meta.term.name);
            };
            let term_list_selection = self.term_list.show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                if !self.term_tabs.select(&term_name) {
                    self.term_tabs.push(&self.terms.get(&term_name).unwrap());
                    self.term_tabs.select(&term_name);
                }
            }
        });

        self.term_tabs.show(ctx, &mut self.terms)
    }
}
