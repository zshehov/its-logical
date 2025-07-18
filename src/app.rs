use std::{collections::HashMap, path::PathBuf};

use its_logical::knowledge::store::in_memory::InMemoryTerms;
use its_logical::knowledge::store::persistent::TermsWithEngine;
use its_logical::knowledge::store::Load;
use its_logical::knowledge::{model::fat_term::parse_fat_term, store::TermsStore};

pub struct ItsLogicalApp<T: TermsStore> {
    ui: crate::ui::App<T>,
}

const SCALE_FACTOR: f32 = 1.2;

impl ItsLogicalApp<TermsWithEngine> {
    pub fn new(c: &eframe::CreationContext<'_>, knowledge_path: PathBuf) -> Self {
        let mut style = (*c.egui_ctx.style()).clone();

        for (_, font) in style.text_styles.iter_mut() {
            font.size *= SCALE_FACTOR;
        }
        c.egui_ctx.set_style(style);

        Self {
            ui: crate::ui::App::new(TermsWithEngine::load(&knowledge_path), knowledge_path),
        }
    }
}

impl ItsLogicalApp<InMemoryTerms> {
    #[allow(dead_code)] // keep this for the examples for now
    pub fn new(c: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*c.egui_ctx.style()).clone();

        for (_, font) in style.text_styles.iter_mut() {
            font.size *= SCALE_FACTOR;
        }
        c.egui_ctx.set_style(style);

        let (_, mother) = parse_fat_term(
            r"% -mother a mother is a parent that's female
% @arg MotherName the name of the mother
% @arg ChildName the name of the child
% @see 
mother(Siika,Mircho).
mother(Stefka,Petko).
mother(Cecka,Krustio).
mother(Mother,Child):-parent(Mother,Child),female(Mother)
",
        )
        .unwrap();
        let (_, father) = parse_fat_term(
            r"% -father a father is a parent that's male
% @arg FatherName the name of the father
% @arg ChildName the name of the child
% @see 
father(Stefan,Petko).
father(Hristo,Stoichko).
father(Father,Child):-parent(Father,Child),male(Father)
",
        )
        .unwrap();
        let (_, male) = parse_fat_term(
            r"% -male is one of the genders that has XY chromosomes
% @arg Name the name of the person
% @see 
male(stefan).
male(petko).
",
        )
        .unwrap();
        Self {
            ui: crate::ui::App::new(
                InMemoryTerms::new(HashMap::from([
                    ("mother".to_string(), mother),
                    ("father".to_string(), father),
                    ("male".to_string(), male),
                ])),
                // in-memory mode doesn't care about base directory
                PathBuf::new(),
            ),
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

impl eframe::App for ItsLogicalApp<TermsWithEngine> {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx)
    }
}
