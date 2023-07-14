use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    rc::Rc,
};

use tracing::debug;

use crate::{
    term_knowledge_base::{DeleteKnowledgeBase, PutKnowledgeBase},
    ui::widgets::{
        tabs::Tabs,
        term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
    },
};

pub(crate) fn finish(
    tabs: &mut Tabs,
    terms: &mut (impl PutKnowledgeBase + DeleteKnowledgeBase),
    is_delete: bool,
    two_phase_commit: Rc<RefCell<TwoPhaseCommit>>,
) {
    debug!("finished commit");

    if two_phase_commit.borrow().waiting_for().len() > 0 {
        debug!("NOT ALL ARE CONFIRMED YET");
    } else {
        let mut relevant: HashSet<String> =
            HashSet::from_iter(two_phase_commit.borrow().iter_approved());
        let mut queue: VecDeque<String> = VecDeque::from_iter(relevant.iter().cloned());

        // BFS through all approved related terms
        while let Some(entry) = queue.pop_front() {
            let new_entries: Vec<String> = tabs
                .get(&entry)
                .expect("relevant terms must be opened in the tabs")
                .two_phase_commit
                .as_ref()
                .expect("relevant terms must be a part of the two phase commit")
                .borrow()
                .iter_approved()
                .filter(|term| !relevant.contains(term))
                .collect();

            queue.extend(new_entries.clone());
            relevant.extend(new_entries);
        }

        let origin = two_phase_commit.borrow().origin();

        if !is_delete {
            relevant.insert(origin);
        } else {
            terms.delete(&origin);
            tabs.close(&origin);
        }

        let relevant: Vec<String> = relevant.into_iter().collect();
        for relevant_term_screen in tabs.borrow_mut(&relevant) {
            let latest_term = relevant_term_screen.extract_term();
            *relevant_term_screen = TermScreen::new(&latest_term, false);
            terms.put(&latest_term.meta.term.name.clone(), latest_term);
        }
    }
}
