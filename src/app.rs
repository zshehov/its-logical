use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    model::fat_term::{parse_fat_term, FatTerm},
    term_knowledge_base::{InMemoryTerms, PersistentMemoryTerms, TermsKnowledgeBase},
};

pub struct ItsLogicalApp<T: TermsKnowledgeBase> {
    ui: crate::ui::App<T>,
}

impl ItsLogicalApp<InMemoryTerms> {
    pub fn new() -> Self {
        let (_, mother) = parse_fat_term(
            r"%! mother a mother is a parent that's female
% @arg MotherName the name of the mother
% @arg ChildName the name of the child
mother(Siika,Mircho).
mother(Stefka,Petko).
mother(Cecka,Krustio).
mother(Mother,Child):-parent(Mother,Child),female(Mother)
",
        )
        .unwrap();
        let (_, father) = parse_fat_term(
            r"%! father a father is a parent that's male
% @arg FatherName the name of the father
% @arg ChildName the name of the child
father(Stefan,Petko).
father(Hristo,Stoichko).
father(Father,Child):-parent(Father,Child),male(Father)
",
        )
        .unwrap();
        let (_, male) = parse_fat_term(
            r"%! male is one of the genders that has XY chromosomes
% @arg Name the name of the person
male(stefan).
male(petko).
",
        )
        .unwrap();
        Self {
            ui: crate::ui::App::new(InMemoryTerms::new(HashMap::from([
                ("mother".to_string(), mother),
                ("father".to_string(), father),
                ("male".to_string(), male),
            ]))),
        }
    }
}

impl ItsLogicalApp<PersistentMemoryTerms> {
    pub fn new() -> Self {
        Self {
            ui: crate::ui::App::new(PersistentMemoryTerms::new(&PathBuf::from(
                "/Users/zdravko/knowledge",
            ))),
        }
    }
}

impl eframe::App for ItsLogicalApp<InMemoryTerms> {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx)
    }
}
impl eframe::App for ItsLogicalApp<PersistentMemoryTerms> {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx)
    }
}
