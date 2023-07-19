use std::{cell::RefCell, rc::Rc};

use crate::{
    changes::ArgsChange,
    model::fat_term::FatTerm,
    term_knowledge_base::GetKnowledgeBase,
    ui::widgets::{
        tabs::{
            commit_tabs::{two_phase_commit::TwoPhaseCommit, CommitTabs},
            term_tabs::TermTabs,
            Tabs,
        },
        term_screen::TermScreen,
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

impl TermHolder for Rc<RefCell<TwoPhaseCommit>> {
    fn get(&self) -> FatTerm {
        self.borrow().term.get_pits().latest().extract_term()
    }

    fn put(&mut self, source: &str, args_changes: &[ArgsChange], term: &FatTerm) {
        self.borrow_mut()
            .term
            .get_pits_mut()
            .0
            .push_pit(args_changes, term, source);
        let pits_count = self.borrow().term.get_pits().len();
        self.borrow_mut().term.choose_pit(pits_count - 1);
    }
}

pub(crate) struct TabsWithLoading<'a, T: GetKnowledgeBase> {
    source_tabs: &'a mut TermTabs<TermScreen>,
    commit_tabs: &'a mut TermTabs<Rc<RefCell<TwoPhaseCommit>>>,
    load_source: &'a T,
}

impl<'a, T: GetKnowledgeBase> TabsWithLoading<'a, T> {
    pub(crate) fn new(tabs: &'a mut Tabs, load_source: &'a T) -> Self {
        Self {
            source_tabs: &mut tabs.term_tabs,
            commit_tabs: &mut tabs.commit_tabs.get_or_insert(CommitTabs::new()).tabs,
            load_source,
        }
    }
}

impl<'a, T: GetKnowledgeBase> Loaded for TabsWithLoading<'a, T> {
    type TermHolder = Rc<RefCell<TwoPhaseCommit>>;

    fn borrow_mut<'b>(
        &'b mut self,
        initiator_name: &str,
        term_names: &[String],
    ) -> Result<(&'b mut Self::TermHolder, Vec<&mut Self::TermHolder>), &'static str> {
        if term_names
            .iter()
            .any(|affected_name| match self.source_tabs.get(affected_name) {
                Some(affected_term_screen) => !affected_term_screen.is_ready_for_change(),
                None => false,
            })
        {
            return Err(
                "There is a term screens that is not ready to be included in a 2 phase commit",
            );
        }

        if self.commit_tabs.get(initiator_name).is_none() {
            // move the ownership from the term_screen tabs to the two_phase_commit tabs
            let initiator = self
                .source_tabs
                .close(initiator_name)
                .expect("initiator must be opened");
            self.commit_tabs.push(&initiator.extract_term());
        }

        for t in term_names {
            if self.commit_tabs.get(t).is_none() {
                self.commit_tabs.push(
                    &self
                        .source_tabs
                        .close(t)
                        .map(|x| x.extract_term())
                        .unwrap_or_else(|| self.load_source.get(t).unwrap()),
                );
            }
        }
        // a bit of a UI touch:
        self.commit_tabs.select(initiator_name);

        let mut with_initiator: Vec<String> = Vec::with_capacity(term_names.len() + 1);
        with_initiator.extend_from_slice(term_names);
        with_initiator.push(initiator_name.to_owned());

        let mut all_term_screens = self.commit_tabs.borrow_mut(&with_initiator);
        let initiator = all_term_screens.swap_remove(
            all_term_screens
                .iter()
                .position(|x| x.borrow().term.name() == initiator_name)
                .unwrap(),
        );

        add_approvers(initiator, &mut all_term_screens);
        Ok((initiator, all_term_screens))
    }
}
