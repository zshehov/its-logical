use std::path::PathBuf;

use egui::Context;
use tracing::debug;

use crate::{
    model::fat_term::FatTerm,
    term_knowledge_base::{LoadKnowledgeBase, TermsKnowledgeBase},
};

use self::widgets::{load_module_menu::LoadModuleMenu, tabs::Tabs, terms_list::TermList};

mod changes_handling;
mod widgets;

pub struct App<T: TermsKnowledgeBase> {
    load_menu: LoadModuleMenu,
    term_tabs: Tabs,
    term_list: TermList,
    terms: T,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase + LoadKnowledgeBase<KnowledgeBase = T>,
{
    pub fn new(terms: T, knowledge_path: PathBuf) -> Self {
        Self {
            term_tabs: Tabs::default(),
            term_list: TermList::new(),
            terms,
            load_menu: LoadModuleMenu::new(knowledge_path),
        }
    }
}

impl<T> App<T>
where
    T: TermsKnowledgeBase + LoadKnowledgeBase<KnowledgeBase = T>,
{
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
            ui.separator();
            ui.vertical_centered_justified(|ui| {
                if let Some(module_path) = self.load_menu.show(ui) {
                    self.terms = T::load(&module_path);
                }
            });
        });

        self.term_tabs.show(ctx, &mut self.terms)
    }
}
