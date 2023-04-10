#[derive(PartialEq,Clone)]
pub(crate) struct Tab {
    pub(crate) name: String,
    pub(crate) kind: TabKind,
}

#[derive(PartialEq,Clone)]
pub enum TabKind {
    Ask,
    Term,
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    current_tab: &mut Tab,
    term_tabs: impl Iterator<Item = Tab>,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(
            current_tab,
            Tab {
                name: "Ask".to_owned(),
                kind: TabKind::Ask,
            },
            egui::RichText::new("Ask").strong(),
        );
        ui.separator();

        for tab in term_tabs {
            let tab_name = tab.name.clone();
            ui.selectable_value(current_tab, tab, tab_name);
        }
    });
}
