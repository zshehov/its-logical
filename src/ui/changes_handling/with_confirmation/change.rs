use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::tabs::Tabs,
};

pub(crate) fn propagate(
    term_tabs: &mut Tabs,
    terms: &impl TermsKnowledgeBase,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let mut loaded_term_screens =
        super::setup_confirmation(term_tabs, terms, original_term, affected);

    let updates = changes::propagation::apply(
        original_term,
        arg_changes,
        updated_term,
        &loaded_term_screens,
    );
    let mut updates = super::with_empty_args_changes(updates);

    updates.insert(
        original_term.meta.term.name.clone(),
        (arg_changes.to_vec(), updated_term.to_owned()),
    );

    super::push_updated_pits(
        updates,
        &original_term.meta.term.name,
        &mut loaded_term_screens,
    );
}
