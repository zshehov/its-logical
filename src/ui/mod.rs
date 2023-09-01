use its_logical::knowledge::model::fat_term::FatTerm;
use its_logical::knowledge::store::{Load, TermsStore};
use std::path::PathBuf;

use egui::Context;
use tracing::debug;

mod load_module_menu;
mod tabs;
mod term_screen;
mod terms_list;
mod widgets;

pub struct App<T: TermsStore> {
    load_menu: load_module_menu::LoadModuleMenu,
    tabs: tabs::Tabs,
    term_list: terms_list::TermList,
    terms: T,
}

impl<T> App<T>
where
    T: TermsStore + Load<Store = T>,
{
    pub fn new(terms: T, knowledge_path: PathBuf) -> Self {
        Self {
            tabs: tabs::Tabs::default(),
            term_list: terms_list::TermList::new(),
            terms,
            load_menu: load_module_menu::LoadModuleMenu::new(knowledge_path),
        }
    }
}

impl<T> App<T>
where
    T: TermsStore + Load<Store = T>,
{
    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            if let Some(output) = self.term_list.show(ui, self.terms.keys().iter()) {
                match output {
                    terms_list::TermListOutput::AddTerm(new_term_name) => {
                        let new_term = FatTerm::default();
                        if self.tabs.select(&new_term.meta.term.name) {
                            debug!("unfinished term creation present");
                            return;
                        }
                        self.tabs.push(&new_term);
                        self.tabs.select(&new_term.meta.term.name);

                        let new_term_screen = self
                            .tabs
                            .get_mut(&new_term.meta.term.name)
                            .expect("the newly created term was just added");

                        match new_term_screen {
                            crate::terms_cache::TermHolder::Normal(s) => {
                                s.start_changes();

                                let (_, editing) = s.get_pits_mut();
                                editing.expect("a").set_name(&new_term_name);
                            }
                            crate::terms_cache::TermHolder::TwoPhase(_) => unreachable!(),
                        }
                    }
                    terms_list::TermListOutput::SelectedTerm(selected_term) => {
                        if !self.tabs.select(&selected_term) {
                            self.tabs.push(&self.terms.get(&selected_term).unwrap());
                            self.tabs.select(&selected_term);
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

        self.tabs.show(ctx, &mut self.terms)
    }
}
