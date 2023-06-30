use tracing::debug;

use crate::{
    changes::{self, ArgsChange},
    model::fat_term::FatTerm,
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::{
        tabs::Tabs,
        term_screen::{term_screen_pit::TermScreenPIT, TermScreen},
    },
};

pub(crate) fn propagate(
    tabs: &mut Tabs,
    terms: &mut impl TermsKnowledgeBase,
    original_term: &FatTerm,
    arg_changes: &[ArgsChange],
    updated_term: &FatTerm,
    affected: &[String],
) {
    let term_name = original_term.meta.term.name.clone();
    debug!("Direct change propagation");
    let mut affected_terms = changes::propagation::apply(
        &original_term,
        &arg_changes,
        &updated_term,
        &super::TermsAdapter::new(terms),
    );
    affected_terms.insert(term_name.clone(), updated_term.to_owned());
    super::update_persisted(terms, affected_terms);

    let updated_term_tab = tabs.get_mut(&term_name).unwrap();
    *updated_term_tab = TermScreen::new(&updated_term, false);

    let update_pit = |pit: &mut TermScreenPIT| {
        let with_applied = changes::propagation::apply(
            &original_term,
            &arg_changes,
            &updated_term,
            &super::SingleTerm {
                term: pit.extract_term(),
            },
        )
        .get(&pit.name())
        .unwrap()
        .to_owned();

        *pit = TermScreenPIT::new(&with_applied);
    };

    super::update_loaded(tabs, affected, update_pit);
}
