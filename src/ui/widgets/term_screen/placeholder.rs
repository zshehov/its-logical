use std::collections::HashSet;

use crate::{
    model::{
        comment::name_description::NameDescription,
        term::{args_binding::ArgsBinding, bound_term::BoundTerm, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
    ui::widgets::drag_and_drop::DragAndDrop,
};

struct HeadPlaceholder {
    binding: Vec<String>,
}

impl HeadPlaceholder {
    fn new() -> Self {
        Self { binding: vec![] }
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
    pub(crate) fn new() -> Self {
        Self {
            head: HeadPlaceholder::new(),
        }
    }
    pub(crate) fn show<'a>(
        &mut self,
        ui: &mut egui::Ui,
        term_name: &str,
        template: impl ExactSizeIterator<Item = &'a NameDescription>,
    ) -> Option<ArgsBinding> {
        self.head.show(ui, term_name, template);
        if ui.small_button("+").clicked() {
            let mut empty_fact_placeholder = FactPlaceholder::new();
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
    external_terms: HashSet<String>,
}

impl RulePlaceholder {
    pub(crate) fn new() -> Self {
        Self {
            head: HeadPlaceholder::new(),
            body: DragAndDrop::new(vec![("".to_string(), vec![])])
                .with_create_item(Box::new(|| ("".to_string(), vec![]))),
            external_terms: HashSet::new(),
        }
    }
    pub(crate) fn show<'a, T: TermsKnowledgeBase>(
        &mut self,
        ui: &mut egui::Ui,
        term_name: &str,
        terms_knowledge_base: &T,
        template: impl ExactSizeIterator<Item = &'a NameDescription>,
    ) -> Option<Rule> {
        self.head.show(ui, term_name, template);
        ui.label(egui::RichText::new("if").weak());

        let mut term_added_to_body = None;
        self.body.show(ui, |s, ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut s.0)
                            .clip_text(false)
                            .desired_width(0.0),
                    )
                    .lost_focus()
                {
                    // TODO: handle the None here
                    let t = terms_knowledge_base.get(&s.0).unwrap();
                    s.1 = vec!["".to_string(); t.meta.args.len()];
                    term_added_to_body = Some(t.meta.term.name.to_owned());
                }
                let mut added_once = false;
                ui.label(egui::RichText::new("(").weak());
                for param in &mut s.1 {
                    if added_once {
                        ui.label(egui::RichText::new(", ").weak());
                    }
                    ui.add(
                        egui::TextEdit::singleline(param)
                            .clip_text(false)
                            .desired_width(SINGLE_CHAR_WIDTH)
                            .hint_text("X"),
                    );
                    added_once = true
                }
                ui.label(egui::RichText::new(")").weak());
            });
        });
        if let Some(term_added_to_body) = term_added_to_body {
            self.external_terms.insert(term_added_to_body);
        }

        if ui.small_button("add rule").clicked() {
            let mut empty_rule_placeholder = RulePlaceholder::new();

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
                if name == "" {
                    return None;
                }

                Some(BoundTerm {
                    name: name.to_owned(),
                    arg_bindings: crate::model::term::args_binding::ArgsBinding {
                        binding: args.to_owned(),
                    },
                })
            })
            .collect();

        Rule {
            arg_bindings: crate::model::term::args_binding::ArgsBinding {
                binding: head_binding.binding,
            },
            body: body_bindings,
        }
    }
}

// TODO: get this from the framework if possible
const SINGLE_CHAR_WIDTH: f32 = 11.0;
