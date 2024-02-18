use std::{cell::RefCell, rc::Rc};

use its_logical::knowledge::{
    engine::{ConsultResult, Engine},
    store::{Get, Keys},
};
use its_logical::knowledge::model::comment::name_description::NameDescription;
use its_logical::knowledge::model::term::args_binding::ArgsBinding;
use its_logical::knowledge::model::term::bound_term::BoundTerm;
use its_logical::knowledge::store::Consult;

use crate::suggestions::FuzzySuggestions;
use crate::ui::widgets::popup_suggestions;

use self::growable_table::GrowableTable;

mod growable_table;

// TODO: move under term_tabs module
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

    fn extract_arguments(&self) -> impl Iterator<Item=String> + '_ {
        self.anchors
            .iter()
            .zip(self.args_initial.iter())
            .map(|(x, y)| {
                if let Some(anchor) = x {
                    return anchor.to_owned();
                }
                y.name.clone()
            })
    }
}

const CONSULT_LIMIT: usize = 10;

impl Ask {
    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        terms: &mut (impl Get + Keys + Consult),
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
                    let bound_term = BoundTerm::new(
                        &self.term_name,
                        ArgsBinding::new(&self.extract_arguments().collect::<Vec<String>>()),
                    );
                    let consult = terms.consult(&bound_term);

                    for binding in consult {
                        let mut with_anchors = bound_term.arg_bindings.binding.clone();
                        with_anchors.iter_mut().for_each(|x|{
                            if let Some(bound_value) = binding.get(x) {
                                *x = bound_value.to_owned()
                            }
                        });

                        self.results.grow(ui, &with_anchors);
                    }
                }
            });
            ui.separator();
        }
    }
}
