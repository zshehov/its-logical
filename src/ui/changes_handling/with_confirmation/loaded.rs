use std::{cell::RefCell, rc::Rc};

use crate::{
    changes::ArgsChange,
    model::fat_term::FatTerm,
    term_knowledge_base::GetKnowledgeBase,
    ui::widgets::{
        tabs::Tabs,
        term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
    },
};

use super::add_approvers;

pub(crate) trait TermHolder {
    fn get(&self) -> FatTerm;
    fn put(&mut self, source: &str, args_changes: &[ArgsChange], term_after_changes: &FatTerm);
}

pub(crate) trait Loaded {
    type TermHolder: TermHolder;
    fn borrow_mut(
        &mut self,
        initiator_name: &str,
        term_names: &[String],
    ) -> Result<(&mut Self::TermHolder, Vec<&mut Self::TermHolder>), &'static str>;
}

impl TermHolder for TermScreen {
    fn get(&self) -> FatTerm {
        self.get_pits().latest().extract_term()
    }

    fn put(&mut self, source: &str, args_changes: &[ArgsChange], term: &FatTerm) {
        self.get_pits_mut().0.push_pit(args_changes, term, source);
        self.choose_pit(self.get_pits().len() - 1);
    }
}

pub(crate) struct TabsWithLoading<'a, T: GetKnowledgeBase> {
    tabs: &'a mut Tabs,
    load_source: &'a T,
}

impl<'a, T: GetKnowledgeBase> TabsWithLoading<'a, T> {
    pub(crate) fn new(tabs: &'a mut Tabs, load_source: &'a T) -> Self {
        Self { tabs, load_source }
    }
}

impl<'a, T: GetKnowledgeBase> Loaded for TabsWithLoading<'a, T> {
    type TermHolder = TermScreen;

    fn borrow_mut<'b>(
        &'b mut self,
        initiator_name: &str,
        term_names: &[String],
    ) -> Result<(&'b mut Self::TermHolder, Vec<&mut Self::TermHolder>), &'static str> {
        let two_phase_commit = Rc::clone(
            self.tabs
                .get_mut(initiator_name)
                .expect("a change is coming from an opened term screen")
                .two_phase_commit
                .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                    initiator_name,
                    true,
                )))),
        );

        let (mut affected, initiator) = open_affected_in_two_phase_commit(
            self.tabs,
            self.load_source,
            &two_phase_commit.borrow(),
            initiator_name,
            term_names,
        )
        .unwrap();

        add_approvers(&two_phase_commit, &mut affected);
        Ok((initiator, affected))
    }
}

fn open_affected_in_two_phase_commit<'a>(
    tabs: &'a mut Tabs,
    terms: &impl GetKnowledgeBase,
    two_phase_commit: &TwoPhaseCommit,
    initiator: &str,
    affected: &[String],
) -> Option<(Vec<&'a mut TermScreen>, &'a mut TermScreen)> {
    if affected
        .iter()
        .any(|affected_name| match tabs.get(affected_name) {
            Some(affected_term_screen) => {
                !affected_term_screen.is_ready_for_change(&two_phase_commit.origin())
            }
            None => false,
        })
    {
        return None;
    }

    for affected_term_name in affected {
        if tabs.get(affected_term_name).is_none() {
            tabs.push(&terms.get(affected_term_name).unwrap());
        }
    }

    let mut with_initiator: Vec<String> = Vec::with_capacity(affected.len() + 1);
    with_initiator.extend_from_slice(affected);
    with_initiator.push(initiator.to_owned());

    let mut all_term_screens = tabs.borrow_mut(&with_initiator);
    let initiator = all_term_screens.swap_remove(
        all_term_screens
            .iter()
            .position(|x| x.name() == initiator)
            .unwrap(),
    );
    Some((all_term_screens, initiator))
}
