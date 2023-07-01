use tracing::debug;

use crate::{
    changes,
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::{tabs::Tabs, term_screen::term_screen_pit::TermScreenPIT},
};

pub(crate) fn propagate(
    tabs: &mut Tabs,
    terms: &mut impl TermsKnowledgeBase,
    original_term: &FatTerm,
    affected: &[String],
) {
    let term_name = original_term.meta.term.name.clone();
    debug!("Direct delete propagation");
    let affected_terms =
        changes::propagation::apply_deletion(original_term, &super::TermsAdapter::new(terms));
    super::update_persisted(terms, affected_terms);
    terms.delete(&term_name);

    tabs.close(&term_name);
    let update_pit = |pit: &mut TermScreenPIT| {
        *pit = TermScreenPIT::new(
            changes::propagation::apply_deletion(
                original_term,
                &super::SingleTerm {
                    term: pit.extract_term(),
                },
            )
            .get(&pit.name())
            .unwrap(),
        );
    };

    super::update_loaded(tabs, affected, update_pit);
}
