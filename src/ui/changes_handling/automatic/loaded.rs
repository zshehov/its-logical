use crate::{
    model::fat_term::FatTerm,
    ui::widgets::{tabs::Tabs, term_screen::term_screen_pit::TermScreenPIT},
};

// Loaded trait represents a container with currently loaded Terms that can be updated with a given
// closure
pub(crate) trait Loaded {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm);
}

impl Loaded for Tabs {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm) {
        if let Some(loaded_term_screen) = self.term_tabs.get_mut(term_name) {
            let (pits, current) = loaded_term_screen.get_pits_mut();

            let update_screen = |term_screen: &mut TermScreenPIT| {
                let before = term_screen.extract_term();
                let after = updator(&before);

                *term_screen = TermScreenPIT::new(&after);
            };

            pits.iter_mut_pits().for_each(update_screen);
            if let Some(current) = current {
                update_screen(current);
                current.start_changes();
            }
        }
    }
}
