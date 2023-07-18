use std::{cell::RefCell, rc::Rc};

use tracing::debug;

use crate::{
    term_knowledge_base::{DeleteKnowledgeBase, PutKnowledgeBase},
    ui::widgets::{
        tabs::term_tabs::TermTabs,
        term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
    },
};

pub(crate) fn finish(
    commit_tabs: &mut TermTabs<Rc<RefCell<TwoPhaseCommit>>>,
    source_tabs: &mut TermTabs<TermScreen>,
    terms: &mut (impl PutKnowledgeBase + DeleteKnowledgeBase),
) {
    debug!("finished commit");

    let mut new_commit_tabs = TermTabs::<Rc<RefCell<TwoPhaseCommit>>>::default();
    std::mem::swap(&mut new_commit_tabs, commit_tabs);

    for screen in new_commit_tabs.screens() {
        let latest = screen.borrow();
        if latest.term.in_deletion() {
            let name = latest.term.name();
            terms.delete(&name);
        } else {
            let latest = latest.term.extract_term();
            source_tabs.push(&latest);
            terms
                .put(&latest.meta.term.name.clone(), latest)
                .expect("putting a term should not fail");
        }
    }
}
