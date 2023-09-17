use its_logical::{
    changes::{
        change::{Apply as _, Change},
        deletion::Deletion,
    },
    knowledge::{self, model::fat_term::FatTerm},
};

use super::{NamedTerm, TermHolder, TermsCache, TwoPhaseTerm};

pub(crate) trait Apply {
    fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm);
}

// convenience impl so that a TermsCache can be passed to change applications
impl<T, K> knowledge::store::Get for TermsCache<T, K>
where
    T: NamedTerm,
    K: TwoPhaseTerm,
{
    fn get(&self, term_name: &str) -> Option<FatTerm> {
        self.get(term_name).map(|term| match term {
            TermHolder::Normal(t) => t.term(),
            TermHolder::TwoPhase(t) => t.term(),
        })
    }
}

impl<T, K> TermsCache<T, K>
where
    T: NamedTerm + Apply,
    K: TwoPhaseTerm + Apply,
{
    // should not be called when the change source is a part of a Two Phase Commit
    // TODO: return error instead of panic
    pub(crate) fn apply_automatic_change(&mut self, change: &Change) {
        if let Some(changed) = self.get_mut(&change.original().meta.term.name) {
            let changed_update = |_: &FatTerm| -> FatTerm { change.changed().to_owned() };
            match changed {
                TermHolder::Normal(t) => t.apply(changed_update),
                TermHolder::TwoPhase(_) => {
                    panic!("change source for automatic change should not be in two phase commit")
                }
            }
        }
        let update_fn = |in_term: &FatTerm| -> FatTerm {
            in_term
                .apply(change)
                .get(&in_term.meta.term.name)
                // the change might not affect the in_term so it needs to be returned as is
                .unwrap_or(in_term)
                .to_owned()
        };
        // It's okay to go for a re-apply for the change source here, since there
        // are no cyclic dependencies, it shouldn't affect the change source again
        for term in &mut self.terms {
            match term {
                super::TermHolder::Normal(t) => t.apply(update_fn),
                super::TermHolder::TwoPhase(t) => t.apply(update_fn),
            }
        }
    }

    pub(crate) fn apply_automatic_deletion(&mut self, term: &FatTerm) {
        let changed_by_deletion = term.apply_deletion(self);
        let update = |t: &FatTerm| -> FatTerm {
            term.apply_deletion(t)
                .get(&t.meta.term.name)
                .unwrap_or(t)
                .to_owned()
        };
        for term_name in changed_by_deletion.keys() {
            if let Some(cached_term) = self.get_mut(term_name) {
                match cached_term {
                    TermHolder::Normal(s) => s.apply(update),
                    TermHolder::TwoPhase(s) => s.apply(update),
                }
            }
        }
        self.remove(&term.meta.term.name);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use its_logical::{
        changes::change::Change,
        knowledge::model::{fat_term::FatTerm, term::rule::parse_rule},
    };

    use super::Apply;
    use crate::terms_cache::{
        change_handling::two_phase_commit::TwoPhaseCommit, NamedTerm, TermHolder, TermsCache,
        TwoPhaseTerm,
    };

    pub(crate) struct ApplyMock {
        term: FatTerm,
    }

    impl Apply for ApplyMock {
        fn apply(&mut self, f: impl Fn(&FatTerm) -> FatTerm) {
            self.term = f(&self.term)
        }
    }
    impl NamedTerm for ApplyMock {
        fn new(term: &FatTerm) -> Self {
            Self {
                term: term.to_owned(),
            }
        }

        fn name(&self) -> String {
            self.term.meta.term.name.clone()
        }

        fn term(&self) -> FatTerm {
            self.term.clone()
        }
    }

    impl TwoPhaseTerm for ApplyMock {
        type Creator = ApplyMock;

        fn from(creator: Self::Creator) -> Self {
            creator
        }

        fn two_phase_commit(&self) -> &Rc<RefCell<TwoPhaseCommit>> {
            todo!()
        }

        fn current_change(&self) -> Change {
            todo!()
        }

        fn before_changes(&self) -> FatTerm {
            todo!()
        }

        fn in_deletion(&self) -> bool {
            todo!()
        }
    }

    #[test]
    fn apply_change_when_cache_is_empty_should_not_crash() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        let mut original = FatTerm::default();
        original.meta.term.name = "original".to_string();

        let mut changed = original.clone();
        changed.meta.term.name = "changed".to_string();

        let change = Change::new(original, &[], changed);

        cache.apply_automatic_change(&change);
        assert_eq!(cache.iter().len(), 0);
    }

    mod setup {
        use super::*;
        fn create_affected() -> FatTerm {
            let mut affected = FatTerm::default();
            affected.meta.term.name = "affected".to_string();
            affected.term.rules.push(
                parse_rule("affected(Some_var):-original(someConst).")
                    .unwrap()
                    .1,
            );
            affected
        }

        pub fn get_change(affected: Option<&str>) -> Change {
            let mut original = FatTerm::default();
            original.meta.term.name = "original".to_string();

            if let Some(affected_name) = affected {
                original.add_referred_by(&affected_name.to_string());
            }

            let mut changed = original.clone();
            changed.meta.term.name = "changed".to_string();

            Change::new(original, &[], changed)
        }

        pub fn get_to_be_deleted(affected: Option<&str>) -> FatTerm {
            let mut original = FatTerm::default();
            original.meta.term.name = "original".to_string();

            if let Some(affected_name) = affected {
                original.add_referred_by(&affected_name.to_string());
            }
            original
        }

        pub(crate) fn add_affected(cache: &mut TermsCache<ApplyMock, ApplyMock>) {
            let affected = create_affected();
            cache.push(&affected);
        }

        pub(crate) fn add_affected_promoted(cache: &mut TermsCache<ApplyMock, ApplyMock>) {
            let mut affected = create_affected();
            affected.meta.term.name = "affected_promoted".to_string();
            cache.push(&affected);
            cache.promote("affected_promoted");
        }

        pub(crate) fn add_unaffected(cache: &mut TermsCache<ApplyMock, ApplyMock>) -> FatTerm {
            let mut unaffected = FatTerm::default();
            unaffected.meta.term.name = "unaffected".to_string();
            cache.push(&unaffected);
            unaffected
        }

        pub(crate) fn add_unaffected_promoted(
            cache: &mut TermsCache<ApplyMock, ApplyMock>,
        ) -> FatTerm {
            let mut unaffected = FatTerm::default();
            unaffected.meta.term.name = "unaffected_promoted".to_string();
            cache.push(&unaffected);
            cache.promote("unaffected_promoted");
            unaffected
        }
    }

    #[test]
    #[should_panic]
    fn apply_change_when_change_source_is_in_commit_should_panic() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();

        let mut original = FatTerm::default();
        original.meta.term.name = "original".to_string();
        cache.push(&original);

        let mut changed = original.clone();
        changed.meta.term.name = "changed".to_string();

        let change = Change::new(original, &[], changed);
        cache.promote("original");

        cache.apply_automatic_change(&change);
    }

    #[test]
    fn apply_change_when_unaffected_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        let unaffected_term = setup::add_unaffected(&mut cache);

        let change = setup::get_change(None);
        cache.apply_automatic_change(&change);

        let unaffected = cache.get("unaffected").expect("should still be present");
        match unaffected {
            TermHolder::Normal(t) => {
                assert_eq!(
                    t.term, unaffected_term,
                    "unaffected terms should not be changed"
                )
            }
            TermHolder::TwoPhase(_) => unreachable!("should not be promoted"),
        }
    }

    #[test]
    fn apply_change_when_unaffected_promoted_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        let unaffected_term = setup::add_unaffected_promoted(&mut cache);

        let change = setup::get_change(None);
        cache.apply_automatic_change(&change);

        let unaffected_promoted = cache
            .get("unaffected_promoted")
            .expect("should still be present");
        match unaffected_promoted {
            TermHolder::Normal(_) => unreachable!("should not be downgraded"),
            TermHolder::TwoPhase(t) => assert_eq!(
                t.term, unaffected_term,
                "unaffected terms should not be changed"
            ),
        }
    }

    #[test]
    fn apply_change_when_affected_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        setup::add_affected(&mut cache);

        let change = setup::get_change(Some("affected"));
        cache.apply_automatic_change(&change);

        let affected = cache.get("affected").expect("should still be present");
        match affected {
            TermHolder::Normal(t) => {
                assert_eq!(
                    t.term.term.rules[0].body[0].name, "changed",
                    "affected term should be changed accordingly"
                )
            }
            TermHolder::TwoPhase(_) => unreachable!("should not be promoted"),
        }
    }

    #[test]
    fn apply_change_when_affected_promoted_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        setup::add_affected_promoted(&mut cache);

        let change = setup::get_change(Some("affected_promoted"));
        cache.apply_automatic_change(&change);

        let affected = cache
            .get("affected_promoted")
            .expect("should still be present");
        match affected {
            TermHolder::Normal(_) => unreachable!("should not be downgraded"),
            TermHolder::TwoPhase(t) => assert_eq!(
                t.term.term.rules[0].body[0].name, "changed",
                "affected term should be changed accordingly"
            ),
        }
    }

    #[test]
    fn apply_change_when_source_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();

        let change = setup::get_change(Some("affected_promoted"));
        cache.push(change.original());

        cache.apply_automatic_change(&change);

        assert!(
            cache.get("original").is_none(),
            "original was changed to changed"
        );
        let change_source = cache.get("changed").expect("should now be renamed");
        match change_source {
            TermHolder::Normal(t) => assert_eq!(t.term.meta.term.name, "changed"),
            TermHolder::TwoPhase(_) => unreachable!("should not be promoted"),
        }
    }

    #[test]
    fn apply_deletion_when_unaffected_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        let unaffected_term = setup::add_unaffected(&mut cache);

        let to_be_deleted = setup::get_to_be_deleted(None);

        cache.apply_automatic_deletion(&to_be_deleted);

        let unaffected = cache.get("unaffected").expect("should still be present");
        match unaffected {
            TermHolder::Normal(t) => {
                assert_eq!(
                    t.term, unaffected_term,
                    "unaffected terms should not be changed"
                )
            }
            TermHolder::TwoPhase(_) => unreachable!("should not be promoted"),
        }
    }

    #[test]
    fn apply_deletion_when_unaffected_promoted_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        let unaffected_term = setup::add_unaffected_promoted(&mut cache);

        let to_be_deleted = setup::get_to_be_deleted(None);

        cache.apply_automatic_deletion(&to_be_deleted);

        let unaffected_promoted = cache
            .get("unaffected_promoted")
            .expect("should still be present");
        match unaffected_promoted {
            TermHolder::Normal(_) => unreachable!("should not be downgraded"),
            TermHolder::TwoPhase(t) => assert_eq!(
                t.term, unaffected_term,
                "unaffected terms should not be changed"
            ),
        }
    }

    #[test]
    fn apply_deletion_when_affected_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        setup::add_affected(&mut cache);

        let to_be_deleted = setup::get_to_be_deleted(Some("affected"));
        cache.apply_automatic_deletion(&to_be_deleted);

        let affected = cache.get("affected").expect("should still be present");
        match affected {
            TermHolder::Normal(t) => {
                assert!(
                    t.term.term.rules.is_empty(),
                    "affected term should be changed accordingly"
                )
            }
            TermHolder::TwoPhase(_) => unreachable!("should not be promoted"),
        }
    }

    #[test]
    fn apply_deletion_when_affected_promoted_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        setup::add_affected_promoted(&mut cache);

        let to_be_deleted = setup::get_to_be_deleted(Some("affected_promoted"));
        cache.apply_automatic_deletion(&to_be_deleted);

        let affected = cache
            .get("affected_promoted")
            .expect("should still be present");
        match affected {
            TermHolder::Normal(_) => unreachable!("should not be downgraded"),
            TermHolder::TwoPhase(t) => assert!(
                t.term.term.rules.is_empty(),
                "affected term should be changed accordingly"
            ),
        }
    }

    #[test]
    fn apply_deletion_when_to_be_deleted_is_present() {
        let mut cache = TermsCache::<ApplyMock, ApplyMock>::default();
        setup::add_affected_promoted(&mut cache);

        let to_be_deleted = setup::get_to_be_deleted(None);
        cache.push(&to_be_deleted);
        cache.apply_automatic_deletion(&to_be_deleted);

        assert!(cache.get("original").is_none());
    }
}
