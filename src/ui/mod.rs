use std::collections::HashSet;

use egui::Context;

use crate::{
    model::{
        fat_term::FatTerm,
        term::{bound_term::BoundTerm, rule::Rule},
    },
    term_knowledge_base::TermsKnowledgeBase,
};

mod widgets;

pub(crate) struct RulePlaceholderState {
    head: Vec<String>,
    body: Vec<(String, Vec<String>)>,
}

impl RulePlaceholderState {
    fn new(args_count: usize) -> Self {
        Self {
            head: vec!["".to_string(); args_count],
            body: vec![("".to_string(), vec![]); 1],
        }
    }
}

pub struct App<T: TermsKnowledgeBase> {
    term_tabs: TermTabs,
    current_tab: widgets::tabs::Tab,
    terms: T,
    current_fat_term: Option<FatTerm>,
    fact_placeholder_state: Vec<String>,
    rule_placeholder_state: RulePlaceholderState,
}

impl<T> App<T>
where
    T: TermsKnowledgeBase,
{
    pub fn new(terms: T) -> Self {
        Self {
            fact_placeholder_state: vec![],
            rule_placeholder_state: RulePlaceholderState::new(0),
            term_tabs: TermTabs::default(),
            current_tab: widgets::tabs::ask_tab(),
            terms,
            current_fat_term: None,
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        let mut tab_change_occurred = false;
        egui::SidePanel::left("terms_panel").show(ctx, |ui| {
            ui.heading("Terms");
            ui.separator();

            let term_list_selection = widgets::terms_list::show(ui, self.terms.keys().iter());

            if let Some(term_name) = term_list_selection {
                self.term_tabs.add(widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                });
                if self.current_tab.name != term_name {
                    tab_change_occurred = true;
                }
                self.current_tab = widgets::tabs::Tab {
                    name: term_name.to_owned(),
                    kind: widgets::tabs::TabKind::Term,
                }
            }
        });
        egui::TopBottomPanel::top("tabs_panel").show(ctx, |ui| {
            tab_change_occurred |= widgets::tabs::show(
                ui,
                &mut self.current_tab,
                self.term_tabs.tabs_vec.iter().cloned(),
            );
            if tab_change_occurred {
                let selected_term = self
                    .current_fat_term
                    .insert(self.terms.get(&self.current_tab.name).unwrap().clone());
                // reset the placeholder
                self.fact_placeholder_state = vec!["".to_string(); selected_term.meta.args.len()];
                self.rule_placeholder_state =
                    RulePlaceholderState::new(selected_term.meta.args.len());
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab.kind {
            widgets::tabs::TabKind::Ask => widgets::ask::show(ui),
            widgets::tabs::TabKind::Term => {
                let change = widgets::term::show(
                    ui,
                    self.current_fat_term.as_ref().unwrap(),
                    &mut self.fact_placeholder_state,
                    &mut self.rule_placeholder_state,
                );
                match change {
                    widgets::term::Change::None => {}
                    widgets::term::Change::NewFact => {
                        let binding = normalize_input_args(self.fact_placeholder_state.iter());
                        self.current_fat_term
                            .as_mut()
                            .unwrap()
                            .term
                            .facts
                            .push(crate::model::term::args_binding::ArgsBinding { binding });

                        self.terms.edit(
                            &self.current_tab.name,
                            self.current_fat_term.as_ref().unwrap(),
                        );

                        // TODO: make this work - probably this state needs to be pulled up to the
                        // App itself
                        tab_change_occurred = true;
                    }
                    widgets::term::Change::NewRule => {
                        let new_rule = placeholder_rule_to_rule(&self.rule_placeholder_state);
                        self.current_fat_term
                            .as_mut()
                            .unwrap()
                            .term
                            .rules
                            .push(new_rule);
                        self.terms.edit(
                            &self.current_tab.name,
                            self.current_fat_term.as_ref().unwrap(),
                        );
                    }
                    widgets::term::Change::RuleBodyLostFocus(term_idx, term_name) => {
                        // TODO: handle this error
                        if let Some(t) = self.terms.get(&term_name) {
                            self.rule_placeholder_state.body[term_idx] =
                                (term_name, vec!["".to_string(); t.meta.args.len()]);
                            self.rule_placeholder_state
                                .body
                                .push(("".to_string(), vec![]));
                        }
                    }
                }
            }
        });
    }
}

#[derive(Default)]
struct TermTabs {
    tabs_vec: Vec<widgets::tabs::Tab>,
    tabs_set: HashSet<String>,
}

impl TermTabs {
    fn add(&mut self, tab: widgets::tabs::Tab) {
        if self.tabs_set.insert(tab.name.to_owned()) {
            self.tabs_vec.push(tab);
        }
    }
}

fn normalize_input_args<'a>(input: impl Iterator<Item = &'a String>) -> Vec<Option<String>> {
    input
        .map(|a| {
            if a == "" {
                return None;
            }
            Some(a.to_string())
        })
        .collect()
}

fn placeholder_rule_to_rule(placeholder: &RulePlaceholderState) -> Rule {
    let head_binding = normalize_input_args(placeholder.head.iter());

    let body_bindings = placeholder
        .body
        .iter()
        .filter_map(|(name, args)| {
            // TODO: maybe do the check that name is not existing here
            if name == "" {
                return None;
            }

            let bound_args = normalize_input_args(args.iter());

            Some(BoundTerm {
                name: name.to_owned(),
                arg_bindings: crate::model::term::args_binding::ArgsBinding {
                    binding: bound_args,
                },
            })
        })
        .collect();

    Rule {
        arg_bindings: crate::model::term::args_binding::ArgsBinding {
            binding: head_binding,
        },
        body: body_bindings,
    }
}
