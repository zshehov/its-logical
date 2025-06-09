
use its_logical::knowledge::model::comment::name_description::NameDescription;
use its_logical::knowledge::model::term::args_binding::ArgsBinding;
use its_logical::knowledge::model::term::bound_term::BoundTerm;
use its_logical::knowledge::store::Consult;
use its_logical::knowledge::store::{Get, Keys};

use crate::suggestions::FuzzySuggestions;
use crate::ui::tabs::ask::table::Table;
use crate::ui::widgets::popup_suggestions;

mod growable_table;
mod table;

// TODO: move under term_tabs module
pub(crate) struct Ask {
    term_name: String,
    anchors: Vec<Option<String>>,
    args_initial: Vec<NameDescription>,
    results: Table,
}

impl Ask {
    pub(crate) fn new() -> Self {
        Self {
            term_name: String::new(),
            anchors: vec![],
            args_initial: vec![],
            results: Table::new(),
        }
    }

    fn extract_anchors(&self) -> Vec<Option<String>> {
        self.anchors.clone()
    }
}

const CONSULT_LIMIT: usize = 10;

impl Ask {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, terms: &mut (impl Get + Keys + Consult)) {
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
            let t = terms
                .get(&self.term_name)
                .expect("selections should only be made from the available terms");
            self.args_initial = t.meta.args;
            self.anchors = vec![None; self.args_initial.len()];
            // reset any results from before
            self.results = Table::new();
            ui.label("Try to consult");
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
                    let bound_term = build_bound_term(&self.term_name, &self.extract_anchors())
                        .expect("couldn't build bound term");

                    let consult = terms.consult(&bound_term);
                    let mut current_results = Vec::with_capacity(consult.len());

                    for binding in consult {
                        let mut with_anchors = bound_term.arg_bindings.binding.clone();
                        with_anchors.iter_mut().for_each(|x| {
                            if let Some(bound_value) = binding.get(x) {
                                *x = bound_value.to_owned()
                            }
                        });
                        current_results.push(with_anchors);
                    }
                    self.results.set_content(current_results);
                }
            });
            ui.separator();
        }
    }
}

fn get_next_random_var_name(current: &str) -> String {
    let (left, last_char) = current.split_at(current.len() - 1);
    let last_char = last_char
        .chars()
        .last()
        .expect("last character is always a single character");

    if last_char.lt(&'Z') {
        let mut result = left.to_string();
        result.push((last_char as u8 + 1) as char);
        return result;
    }
    let mut result = current.to_string();
    result.push('A');
    result
}

fn build_bound_term(term_name: &str, anchors: &[Option<String>]) -> Result<BoundTerm, String> {
    let all_anchors_start_with_lower_case = anchors.iter().flatten().all(|x| {
        x.chars()
            .next()
            .expect("anchors have at least 1 character")
            .is_lowercase()
    });
    if !all_anchors_start_with_lower_case {
        // TODO: maybe just filter out anchors that are upper-cased?
        return Err("there are anchors that would be interpreted as Prolog variables".to_string());
    }

    let mut current_var_name: String = ((b'A' - 1) as char).to_string();
    let term_args: Vec<String> = anchors
        .iter()
        .map(|x| match x {
            None => {
                current_var_name = get_next_random_var_name(&current_var_name);
                current_var_name.clone()
            }
            Some(anchor) => anchor.to_owned(),
        })
        .collect();

    Ok(BoundTerm::new(term_name, ArgsBinding::new(&term_args)))
}
