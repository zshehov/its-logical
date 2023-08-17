use crate::knowledge::model::comment::name_description::NameDescription;
use crate::knowledge::{
    engine::{ConsultResult, Engine},
    store::{Get, Keys},
};
use std::{cell::RefCell, rc::Rc};

use crate::{suggestions::FuzzySuggestions, ui::widgets::popup_suggestions};

use self::growable_table::GrowableTable;

mod growable_table;

pub(crate) struct Ask {
    term_name: String,
    anchors: Vec<Option<String>>,
    args_initial: Vec<NameDescription>,
    results: GrowableTable,
    consult: Option<Rc<RefCell<ConsultResult>>>,
}

impl Ask {
    pub(crate) fn new() -> Self {
        Self {
            term_name: String::new(),
            anchors: vec![],
            args_initial: vec![],
            results: GrowableTable::new(),
            consult: None,
        }
    }

    fn extract_request(&self) -> impl Iterator<Item = &String> {
        self.anchors
            .iter()
            .zip(self.args_initial.iter())
            .map(|(x, y)| {
                if let Some(anchor) = x {
                    return anchor;
                }
                &y.name
            })
    }
}

const CONSULT_LIMIT: usize = 10;

impl Ask {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        engine: &mut impl Engine,
        terms: &(impl Get + Keys),
    ) {
        let term_suggestions = FuzzySuggestions::new(terms.keys().iter().cloned());
        if popup_suggestions::show(
            ui,
            &mut self.term_name,
            |ui, current_val| {
                ui.add(
                    egui::TextEdit::singleline(current_val)
                        .clip_text(false)
                        .font(egui::TextStyle::Heading)
                        .hint_text("Term name")
                        .desired_width(130.0),
                )
            },
            &term_suggestions,
        )
        .changed()
        {
            // TODO: handle the None here
            let t = terms.get(&self.term_name).unwrap();
            self.args_initial = t.meta.args;
            self.anchors = vec![None; self.args_initial.len()];

            self.results = GrowableTable::new();
        }
        ui.separator();

        if !self.args_initial.is_empty() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    for (arg, anchored) in self.args_initial.iter().zip(self.anchors.iter_mut()) {
                        ui.horizontal(|ui| {
                            ui.label(&arg.name).on_hover_text(&arg.desc);
                            match anchored {
                                Some(anchored_arg) => {
                                    ui.add_enabled(false, egui::Button::new("="))
                                        .on_disabled_hover_text(format!(
                                            "{} is anchored to {}",
                                            arg.name, anchored_arg
                                        ));
                                    ui.text_edit_singleline(anchored_arg);
                                    if ui.button("❌").clicked() {
                                        *anchored = None;
                                    }
                                }
                                None => {
                                    if ui
                                        .button("⚓")
                                        .on_hover_text(format!(
                                            "anchor {} to another variable or a constant",
                                            arg.name
                                        ))
                                        .clicked()
                                    {
                                        *anchored = Some(String::new());
                                    }
                                }
                            }
                        });
                    }
                });
                ui.separator();
                if self.results.show(ui) {
                    let consult = engine.ask(
                        &self.term_name,
                        self.extract_request()
                            .cloned()
                            .collect::<Vec<String>>()
                            .as_slice(),
                    );

                    let consult = self.consult.get_or_insert_with(|| Rc::clone(&consult));

                    let mut i = 0;
                    while let Some(next) = &consult.borrow_mut().more() {
                        self.results.grow(ui, next);
                        i += 1;
                        if i >= CONSULT_LIMIT {
                            break;
                        }
                    }
                }
            });
            ui.separator();
        }
    }
}
