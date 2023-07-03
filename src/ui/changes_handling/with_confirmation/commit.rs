use std::{cell::RefCell, rc::Rc};

use tracing::debug;

use crate::{
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::{
        tabs::Tabs,
        term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
    },
};

pub(crate) fn finish(
    tabs: &mut Tabs,
    terms: &mut impl TermsKnowledgeBase,
    is_delete: bool,
    two_phase_commit: Rc<RefCell<TwoPhaseCommit>>,
) {
    debug!("finished commit");

    if two_phase_commit.borrow().waiting_for().len() > 0 {
        debug!("NOT ALL ARE CONFIRMED YET");
    } else {
        // TODO: this should be done recursively
        let mut relevant: Vec<String> = two_phase_commit.borrow().iter_approved().collect();
        let origin = two_phase_commit.borrow().origin();

        if !is_delete {
            relevant.push(origin);
        } else {
            terms.delete(&origin);
            tabs.close(&origin);
        }

        for relevant_term_screen in tabs.borrow_mut(&relevant) {
            let latest_term = relevant_term_screen.extract_term();
            *relevant_term_screen = TermScreen::new(&latest_term, false);
            terms.put(&latest_term.meta.term.name.clone(), latest_term);
        }
    }
}
