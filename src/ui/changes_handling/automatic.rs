use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::KnowledgeBaseError,
    ui::widgets::{tabs::Tabs, term_screen::term_screen_pit::TermScreenPIT},
};

pub(crate) fn propagate(
    persistent: &mut impl PersistentTerms,
    loaded: &mut impl Loaded,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let term_name = original_term.meta.term.name.clone();
    debug!("Direct change propagation");
    let mut affected_terms = changes::propagation::apply(
        original_term,
        arg_changes,
        updated_term,
        // TODO: reuse the trait impl from with_confirmation
        &TermsAdapter::new(persistent),
    );
    affected_terms.insert(term_name.clone(), updated_term.to_owned());

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        changes::propagation::apply(
            original_term,
            arg_changes,
            updated_term,
            &SingleTerm {
                term: in_term.to_owned(),
            },
        )
        .get(&in_term.meta.term.name)
        .unwrap()
        .to_owned()
    };

    loaded.update_with(&term_name, |_| updated_term.clone());
    for affected_term_name in affected {
        loaded.update_with(&affected_term_name, update_fn);
    }
}

pub(crate) fn propagate_deletion(
    persistent: &mut impl PersistentTerms,
    loaded: &mut impl Loaded,
    term: &FatTerm,
) {
    debug!("Direct delete propagation");
    let affected_terms = changes::propagation::apply_deletion(term, &TermsAdapter::new(persistent));

    for (affected_term_name, affected_updated_term) in affected_terms.into_iter() {
        persistent
            .put(&affected_term_name, affected_updated_term)
            .expect("writing to persistence layer should not fail");
    }

    let update_fn = |in_term: &FatTerm| -> FatTerm {
        changes::propagation::apply_deletion(
            term,
            &SingleTerm {
                term: in_term.to_owned(),
            },
        )
        .get(&in_term.meta.term.name)
        .unwrap()
        .to_owned()
    };

    for affected_term_name in changes::propagation::affected_from_deletion(term) {
        loaded.update_with(&affected_term_name, update_fn);
    }
}

pub(crate) trait PersistentTerms {
    fn put(&mut self, term_name: &str, term: FatTerm) -> Result<(), KnowledgeBaseError>;
    fn get(&self, term_name: &str) -> Option<FatTerm>;
}

pub(crate) trait Loaded {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm);
}

struct TermsAdapter<'a, T: PersistentTerms> {
    incoming: &'a T,
}

impl<'a, T: PersistentTerms> TermsAdapter<'a, T> {
    fn new(incoming: &'a T) -> Self {
        Self { incoming }
    }
}

impl<'a, T: PersistentTerms> changes::propagation::Terms for TermsAdapter<'a, T> {
    fn get(&self, term_name: &str) -> Option<crate::model::fat_term::FatTerm> {
        self.incoming.get(term_name)
    }
}

pub(crate) struct SingleTerm {
    term: FatTerm,
}

impl SingleTerm {
    pub(crate) fn new(term: FatTerm) -> Self {
        Self { term }
    }
}

impl changes::propagation::Terms for SingleTerm {
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        if term_name == self.term.meta.term.name {
            Some(self.term.clone())
        } else {
            None
        }
    }
}

impl Loaded for Tabs {
    fn update_with(&mut self, term_name: &str, updator: impl Fn(&FatTerm) -> FatTerm) {
        if let Some(loaded_term_screen) = self.get_mut(term_name) {
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
