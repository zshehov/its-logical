use crate::{
    changes, model::fat_term::FatTerm, term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::tabs::Tabs,
};

pub(crate) fn propagate(tabs: &mut Tabs, terms: &impl TermsKnowledgeBase, term: &FatTerm) {
    let mut loaded_term_screens = super::setup_confirmation(
        tabs,
        terms,
        term,
        &changes::propagation::affected_from_deletion(term),
    );

    let updates = changes::propagation::apply_deletion(term, &loaded_term_screens);
    super::push_updated_pits(
        super::with_empty_args_changes(updates),
        &term.meta.term.name,
        &mut loaded_term_screens,
    );
}
