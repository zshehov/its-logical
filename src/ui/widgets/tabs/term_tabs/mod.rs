use its_logical::knowledge::model::fat_term::FatTerm;
use std::cmp::min;

use screen::Screen;

pub(crate) mod screen;

pub(crate) struct TermTabs<T: Screen> {
    current_tab: Option<usize>,
    screens: Vec<T>,
}

impl<T: Screen> Default for TermTabs<T> {
    fn default() -> Self {
        TermTabs {
            current_tab: None,
            screens: vec![],
        }
    }
}

impl<T: Screen> TermTabs<T> {
    pub(crate) fn new() -> Self {
        Self {
            current_tab: None,
            screens: vec![],
        }
    }

    pub(crate) fn push(&mut self, term: &FatTerm) {
        self.screens.push(T::new(term));
    }

    pub(crate) fn get(&self, term_name: &str) -> Option<&T> {
        if let Some(term_idx) = self.screens.iter().position(|x| x.name() == term_name) {
            return Some(&self.screens[term_idx]);
        }
        None
    }

    pub(crate) fn get_mut(&mut self, term_name: &str) -> Option<&mut T> {
        if let Some(term_idx) = self.screens.iter().position(|x| x.name() == term_name) {
            return Some(&mut self.screens[term_idx]);
        }
        None
    }

    pub(crate) fn borrow_mut(&mut self, names: &[String]) -> Vec<&mut T> {
        let screens = self
            .screens
            .iter_mut()
            .filter(|screen| {
                if names.contains(&screen.name()) {
                    return true;
                }
                false
            })
            .collect();

        screens
    }

    pub(crate) fn close(&mut self, term_name: &str) -> Option<T> {
        if let Some(term_idx) = self.screens.iter().position(|x| x.name() == term_name) {
            return self.close_idx(term_idx);
        }
        None
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.screens.iter()
    }

    pub(crate) fn screens(self) -> Vec<T> {
        self.screens
    }

    fn close_idx(&mut self, idx: usize) -> Option<T> {
        if idx >= self.screens.len() {
            return None;
        }
        if self.screens.len() == 1 {
            self.current_tab = None;
        }
        if let Some(current_idx) = &mut self.current_tab {
            if idx < *current_idx {
                *current_idx -= 1;
            } else {
                *current_idx = min(*current_idx, self.screens.len() - 1 - 1);
            }
        }
        Some(self.screens.remove(idx))
    }
}

// UI related
impl<T: Screen> TermTabs<T> {
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<&mut T> {
        ui.horizontal(|ui| {
            let mut close_idx = None;
            for (idx, screen) in self.screens.iter_mut().enumerate() {
                ui.scope(|ui| {
                    let selectable = ui.selectable_value(
                        &mut self.current_tab,
                        Some(idx),
                        if screen.name() == "" {
                            "untitled*".to_string()
                        } else if !screen.can_close() {
                            screen.name() + "*"
                        } else {
                            screen.name()
                        },
                    );

                    ui.painter().line_segment(
                        [
                            selectable.rect.left_bottom(),
                            selectable.rect.right_bottom(),
                        ],
                        screen.stroke(),
                    );

                    if selectable.secondary_clicked() {
                        close_idx = Some(idx);
                    };
                });
            }
            if let Some(close_idx) = close_idx {
                if !self.screens[close_idx].can_close() {
                    // tab can't be closed - switch to it for the user to see what's going on
                    self.current_tab = Some(close_idx);
                } else {
                    self.close_idx(close_idx);
                }
            }
        });
        if let Some(idx) = self.current_tab {
            return Some(&mut self.screens[idx]);
        }
        None
    }

    pub(crate) fn select(&mut self, term_name: &str) -> bool {
        if let Some(term_idx) = self.screens.iter().position(|x| x.name() == term_name) {
            self.current_tab = Some(term_idx);
            return true;
        }
        false
    }

    pub(crate) fn unselect(&mut self) {
        self.current_tab = None;
    }
}
