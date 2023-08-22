use its_logical::knowledge::{
    model::{
        comment::name_description::NameDescription,
        term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule},
    },
    store::{Get, Keys},
};
use crate::suggestions::FuzzySuggestions;
use egui::RichText;

use crate::ui::widgets::{drag_and_drop::DragAndDrop, popup_suggestions};

struct HeadPlaceholder {
    binding: Vec<String>,
}

impl HeadPlaceholder {
    fn new(args: &[String]) -> Self {
        Self {
            binding: args.to_vec(),
        }
    }

    fn show<'a>(
        &mut self,
        ui: &mut egui::Ui,
        term_name: &str,
        template: impl ExactSizeIterator<Item = &'a NameDescription>,
    ) {
        ui.label(egui::RichText::new(format!("{} (", term_name)).weak());

        let mut added_once = false;
        if template.len() != self.binding.len() {
            self.binding = vec![String::new(); template.len()];
        }

        for (template_param, param) in template.zip(self.binding.iter_mut()) {
            if added_once {
                ui.label(egui::RichText::new(", ").weak());
            }
            ui.add(
                egui::TextEdit::singleline(param)
                    .hint_text(&template_param.name)
                    .clip_text(false)
                    .desired_width(SINGLE_CHAR_WIDTH * template_param.name.len() as f32),
            );
            added_once = true
        }
        ui.label(egui::RichText::new(")").weak());
    }
}

pub(crate) struct FactPlaceholder {
    head: HeadPlaceholder,
}

impl FactPlaceholder {
    pub(crate) fn new(args: &[String]) -> Self {
        Self {
            head: HeadPlaceholder::new(args),
        }
    }
    pub(crate) fn show<'a>(
        &mut self,
        ui: &mut egui::Ui,
        term_name: &str,
        template: impl ExactSizeIterator<Item = &'a NameDescription>,
        finish_button_text: &str,
    ) -> Option<ArgsBinding> {
        self.head.show(ui, term_name, template);
        if ui
            .small_button(RichText::new(finish_button_text).monospace())
            .clicked()
        {
            let mut empty_fact_placeholder = FactPlaceholder::new(&[]);
            // reset the placeholder
            std::mem::swap(&mut empty_fact_placeholder, self);

            return Some(ArgsBinding {
                binding: empty_fact_placeholder.head.binding,
            });
        }
        None
    }
}

pub(crate) struct RulePlaceholder {
    head: HeadPlaceholder,
    body: DragAndDrop<(String, Vec<String>)>,
}

impl RulePlaceholder {
    pub(crate) fn new() -> Self {
        Self {
            head: HeadPlaceholder::new(&[]),
            body: DragAndDrop::new(vec![("".to_string(), vec![])])
                .with_create_item("constructor", Box::new(|| ("".to_string(), vec![]))),
        }
    }
    pub(crate) fn show<'a>(
        &mut self,
        ui: &mut egui::Ui,
        term_name: &str,
        terms_knowledge_base: &(impl Get + Keys),
        template: impl ExactSizeIterator<Item = &'a NameDescription>,
        finish_button_text: &str,
    ) -> Option<Rule> {
        self.head.show(ui, term_name, template);
        ui.label(egui::RichText::new("if").weak());

        let mut term_added_to_body = None;

        let term_suggestions = FuzzySuggestions::new(terms_knowledge_base.keys().iter().cloned());
        let arg_suggestions = FuzzySuggestions::new(
            self.body
                .iter()
                .flat_map(|(_, vecs)| vecs)
                .filter(|&x| !x.is_empty())
                .chain(self.head.binding.iter())
                .cloned(),
        );

        self.body.show(ui, |s, ui| {
            ui.horizontal(|ui| {
                if popup_suggestions::show(
                    ui,
                    &mut s.0,
                    |ui, current_val| {
                        ui.add(
                            egui::TextEdit::singleline(current_val)
                                .clip_text(false)
                                .desired_width(0.0),
                        )
                    },
                    &term_suggestions,
                )
                .changed()
                {
                    // TODO: handle the None here
                    let t = terms_knowledge_base.get(&s.0).unwrap();
                    s.1 = vec!["".to_string(); t.meta.args.len()];
                    term_added_to_body = Some(t.meta.term.name);
                }
                let mut added_once = false;

                ui.label(egui::RichText::new("(").weak());
                for param in &mut s.1 {
                    if added_once {
                        ui.label(egui::RichText::new(", ").weak());
                    }
                    popup_suggestions::show(
                        ui,
                        param,
                        |ui, current_val| {
                            ui.add(
                                egui::TextEdit::singleline(current_val)
                                    .clip_text(false)
                                    .hint_text("X")
                                    .desired_width(SINGLE_CHAR_WIDTH),
                            )
                        },
                        &arg_suggestions,
                    );
                    added_once = true
                }
                ui.label(egui::RichText::new(")").weak());
            });
        });

        if ui
            .small_button(egui::RichText::new(finish_button_text).monospace())
            .clicked()
        {
            let mut empty_rule_placeholder = RulePlaceholder::new();
            empty_rule_placeholder.unlock();

            // reset the rule placeholder
            std::mem::swap(&mut empty_rule_placeholder, self);

            return Some(empty_rule_placeholder.into());
        }
        None
    }

    pub(crate) fn unlock(&mut self) {
        self.body.unlock()
    }
}

impl From<RulePlaceholder> for Rule {
    fn from(placeholder: RulePlaceholder) -> Self {
        let head_binding = placeholder.head;

        let body_bindings = placeholder
            .body
            .iter()
            .filter_map(|(name, args)| {
                // TODO: maybe do the check that name is not existing here
                if name.is_empty() {
                    return None;
                }

                Some(BoundTerm {
                    name: name.to_owned(),
                    arg_bindings: its_logical::knowledge::model::term::args_binding::ArgsBinding {
                        binding: args.to_owned(),
                    },
                })
            })
            .collect();

        Rule {
            head: its_logical::knowledge::model::term::args_binding::ArgsBinding {
                binding: head_binding.binding,
            },
            body: body_bindings,
        }
    }
}
impl From<Rule> for RulePlaceholder {
    fn from(rule: Rule) -> Self {
        let body_bindings = DragAndDrop::new(
            rule.body
                .into_iter()
                .map(|bound_term| {
                    let BoundTerm { name, arg_bindings } = bound_term;
                    (name, arg_bindings.binding)
                })
                .collect(),
        )
        .with_create_item("from rule", Box::new(|| ("".to_string(), vec![])));

        RulePlaceholder {
            head: HeadPlaceholder {
                binding: rule.head.binding,
            },
            body: body_bindings,
        }
    }
}

// TODO: get this from the framework if possible
const SINGLE_CHAR_WIDTH: f32 = 11.0;
