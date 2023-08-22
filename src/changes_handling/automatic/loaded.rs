use its_logical::knowledge::model::fat_term::FatTerm;

use crate::ui::{
    tabs::Tabs,
    term_screen::{term_screen_pit::TermScreenPIT, TermScreen},
};

// Loaded trait represents a container with currently loaded Terms that can be updated with a given
// closure
pub(crate) trait Loaded {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm);
}

impl Loaded for Tabs {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm) {
        if let Some(loaded_term_screen) = self.term_tabs.get_mut(term_name) {
            update_loaded(loaded_term_screen, &updator);
        }

        if let Some(commit_tabs) = &mut self.commit_tabs {
            if let Some(loaded_term_screen) = commit_tabs.tabs.get_mut(term_name) {
                let mut loaded_term_screen = loaded_term_screen.borrow_mut();
                update_loaded(&mut loaded_term_screen.term, &updator);
            }
        }
    }
}

fn update_loaded(screen: &mut TermScreen, updator: impl Fn(&FatTerm) -> FatTerm) {
    let (pits, current) = screen.get_pits_mut();
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
