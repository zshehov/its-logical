use std::collections::HashMap;

pub struct ItsLogicalApp {
    ui: crate::ui::App,
}

impl ItsLogicalApp {
    pub fn new() -> Self {
        Self {
            ui: crate::ui::App::new(HashMap::from([
                (
                    String::from("mother"),
                    crate::model::term::Term::new(
                        "mother",
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
                ),
            ])),
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
