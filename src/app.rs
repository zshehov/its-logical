use std::collections::HashMap;

use crate::model::{
    comment::{comment::Comment, name_description::NameDescription},
    fat_term::FatTerm,
    term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule, term::Term},
};

pub struct ItsLogicalApp {
    ui: crate::ui::App,
}

impl ItsLogicalApp {
    pub fn new() -> Self {
        Self {
            ui: crate::ui::App::new(HashMap::from([(
                "mother".to_string(),
                FatTerm::new(
                    Comment::new(
                        NameDescription::new("mother", "a mother is a parent that's female"),
                        vec![
                            NameDescription::new("MotherName", "the name of the mother"),
                            NameDescription::new("ChildName", "the name of the child"),
                        ],
                    ),
                    Term::new(
                        vec![
                            ArgsBinding {
                                binding: vec![
                                    Some("Siika".to_string()),
                                    Some("Mircho".to_string()),
                                ],
                            },
                            ArgsBinding {
                                binding: vec![
                                    Some("Stefka".to_string()),
                                    Some("Petko".to_string()),
                                ],
                            },
                        ],
                        vec![Rule {
                            arg_bindings: ArgsBinding {
                                binding: vec![
                                    Some("Mother".to_string()),
                                    Some("Child".to_string()),
                                ],
                            },
                            body: vec![
                                BoundTerm {
                                    name: "parent".to_string(),
                                    arg_bindings: ArgsBinding {
                                        binding: vec![
                                            Some("Mother".to_string()),
                                            Some("Child".to_string()),
                                        ],
                                    },
                                },
                                BoundTerm {
                                    name: "female".to_string(),
                                    arg_bindings: ArgsBinding {
                                        binding: vec![Some("Mother".to_string())],
                                    },
                                },
                            ],
                        }],
                    ),
                ), /*
                       String::from("mother"),
                       crate::model::term::Term::new(
                           "a mother is a parent that's female",
                           &["MotherName", "ChildName"],
                           vec![
                               vec![Some("Siika".to_owned()), Some("Mircho".to_owned())],
                               vec![Some("Stefka".to_owned()), Some("Petko".to_owned())],
                           ],
                           vec![(
                               vec![Some("X".to_owned()), Some("Y".to_owned())],
                               "parent(X, Y) and female(X)".to_owned(),
                           )],
                       ),
                   ),
                   (
                       String::from("father"),
                       crate::model::term::Term::new(
                           "father",
                           "a father is a parent that's male",
                           &["FatherName", "ChildName"],
                           vec![
                               vec![Some("Krustio".to_owned()), Some("Mircho".to_owned())],
                               vec![Some("Stefcho".to_owned()), Some("Mitko".to_owned())],
                           ],
                           vec![(
                               vec![Some("X".to_owned()), Some("Y".to_owned())],
                               "parent(X, Y) and male(X)".to_owned(),
                           )],
                       ),
                   ),
                   (
                       String::from("male"),
                       crate::model::term::Term::new(
                           "male",
                           "male is one of the 2 genders",
                           &["PersonName"],
                           vec![
                               vec![Some("Krustio".to_owned())],
                               vec![Some("Mircho".to_owned())],
                               vec![Some("Stefcho".to_owned())],
                               vec![Some("Mitko".to_owned())],
                           ],
                           vec![(
                               vec![Some("PersonName".to_owned())],
                               "chromosomes(PersonName, Chromosomes) and Chromosomes == [X,Y]"
                                   .to_owned(),
                           )],
                       ),
                       */
            )])),
        }
    }
}

impl eframe::App for ItsLogicalApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui.show(ctx)
    }
}
