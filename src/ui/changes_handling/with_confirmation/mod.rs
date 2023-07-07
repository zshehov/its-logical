use std::{cell::RefCell, rc::Rc};

use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::GetKnowledgeBase,
    ui::widgets::{
        tabs::Tabs,
        term_screen::{two_phase_commit::TwoPhaseCommit, TermScreen},
    },
};

pub(crate) mod commit;

pub(crate) fn add_approvers(
    source_two_phase_commit: &Rc<RefCell<TwoPhaseCommit>>,
    approvers: &mut [&mut TermScreen],
) {
    let origin_name = source_two_phase_commit.borrow().origin();

    let mut approvers_names = Vec::with_capacity(approvers.len());
    for approver in approvers {
        debug!("Adding approver {}", approver.name());
        approver
            .two_phase_commit
            .get_or_insert(Rc::new(RefCell::new(TwoPhaseCommit::new(
                &origin_name,
                false,
            ))))
            .borrow_mut()
            .add_approval_waiter(Rc::clone(source_two_phase_commit));
        approvers_names.push(approver.name());
    }
    source_two_phase_commit
        .borrow_mut()
        .append_approval_from(&approvers_names);
}

pub(crate) trait TermHolder {
    fn get(&self) -> FatTerm;
    fn put(&mut self, source: &str, args_changes: &[ArgsChange], term: &FatTerm);
}

pub(crate) trait Loaded {
    type TermHolder: TermHolder;
    fn borrow_mut(
        &mut self,
        initiator_name: &str,
        term_names: &[String],
    ) -> Result<(&mut Self::TermHolder, Vec<&mut Self::TermHolder>), &'static str>;
}

pub(crate) fn propagate(
    mut loaded: impl Loaded,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let (initiator, affected) = loaded
        .borrow_mut(&original_term.meta.term.name, affected)
        .expect("[TODO] inability to load is not handled");

    let updates =
        changes::propagation::apply(original_term, arg_changes, updated_term, &affected.as_ref());

    initiator.put(&updated_term.meta.term.name, arg_changes, updated_term);
    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&updated_term.meta.term.name, &vec![], updated);
        }
    }
}

pub(crate) fn propagate_deletion(mut loaded: impl Loaded, term: &FatTerm) {
    let (_, affected) = loaded
        .borrow_mut(
            &term.meta.term.name,
            &changes::propagation::affected_from_deletion(term),
        )
        .expect("[TODO] inability to load is not handled");

    let updates = changes::propagation::apply_deletion(term, &affected.as_ref());

    for affected_term in affected {
        if let Some(updated) = updates.get(&affected_term.get().meta.term.name) {
            affected_term.put(&term.meta.term.name, &vec![], updated);
        }
    }
}

impl<H> GetKnowledgeBase for &[&mut H]
where
    H: TermHolder,
{
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        for term in self.iter() {
            let term = term.get();
            if term.meta.term.name == term_name {
                return Some(term.clone());
            }
        }
        None
    }
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

        let (mut affected, initiator) = validate_two_phase(
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

fn validate_two_phase<'a>(
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
