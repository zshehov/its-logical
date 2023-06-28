use std::{cmp::min, hash::Hash};

use eframe::epaint::RectShape;
use egui::{CursorIcon, Id, LayerId, Order, Rect, Sense, Shape, Ui, Vec2};

pub(crate) use self::change_tracking_list::Change;
use self::change_tracking_list::ChangeTrackingVec;

mod change_tracking_list;

pub(crate) struct DragAndDrop<T: Hash + Clone + Eq> {
    active: bool,
    items: ChangeTrackingVec<T>,
    bottoms: Vec<f32>,
    create_item: Option<Box<dyn Fn() -> T>>,
    default_value_id: Option<Id>,
}
const ID_SOURCE: &str = "drag_and_drop";

impl<T: Hash + Clone + Eq> DragAndDrop<T> {
    pub(crate) fn new(items: Vec<T>) -> Self {
        let items_len = items.len();
        Self {
            active: false,
            items: ChangeTrackingVec::new(items),
            create_item: None,
            bottoms: vec![0.0; items_len],
            default_value_id: None,
        }
    }
    pub(crate) fn with_create_item(mut self, create_item: Box<dyn Fn() -> T>) -> Self {
        let prototype = create_item();
        self.create_item = Some(create_item);
        self.default_value_id = Some(Id::new(ID_SOURCE).with(prototype));
        self
    }

    pub(crate) fn unlock(&mut self) {
        self.active = true;
    }

    pub(crate) fn lock(&mut self) -> Vec<Change<T>> {
        self.active = false;
        return self.items.get_current_changes();
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub(crate) fn remove(&mut self, idx: usize) -> T {
        self.bottoms.pop();
        self.items.remove(idx)
    }

    // TODO: maybe return a Result here, since failures may occur (repeating item id)
    pub(crate) fn push(&mut self, item: T) {
        self.items.push(item);
        self.bottoms.push(0.0);
    }

    pub(crate) fn show(&mut self, ui: &mut Ui, mut show_item: impl FnMut(&mut T, &mut Ui)) {
        let margin = Vec2::splat(4.0);

        ui.vertical(|ui| {
            let outer_rect_bounds = ui.available_rect_before_wrap();
            let inner_rect = outer_rect_bounds.shrink2(margin);
            let where_to_put_background = ui.painter().add(Shape::Noop);

            let mut content_ui = ui.child_ui(inner_rect, *ui.layout());

            content_ui.vertical(|ui| {
                match self.active {
                    true => {
                        self.fix_dragged_item_position(ui);
                        let mut default_item_present = false;
                        let mut item_for_deletion_idx = None;
                        for (idx, (item, bottom)) in self
                            .items
                            .iter_mut()
                            .zip(self.bottoms.iter_mut())
                            .enumerate()
                        {
                            let item_id = Id::new(ID_SOURCE).with(&item);
                            if let Some(default_item_id) = self.default_value_id {
                                if item_id == default_item_id {
                                    default_item_present = true;
                                }
                            }

                            let mut render_entry = |ui: &mut Ui| -> egui::Response {
                                ui.horizontal(|ui| {
                                    // the grab should only happen on the "::" part of the item
                                    let scoped_handle = ui
                                        .scope(|ui| {
                                            ui.label(egui::RichText::new("∷").heading().monospace())
                                        })
                                        .response;
                                    show_item(item, ui);
                                    if ui.small_button("-").clicked() {
                                        item_for_deletion_idx = Some(idx);
                                    }
                                    scoped_handle
                                })
                                .inner
                            };

                            match ui.memory(|mem| mem.is_being_dragged(item_id)) {
                                true => {
                                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

                                    // Paint the body to a new layer:
                                    let layer_id = LayerId::new(Order::Tooltip, item_id);
                                    let response =
                                        ui.with_layer_id(layer_id, render_entry).response;
                                    *bottom = response.rect.bottom();

                                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                        let delta = pointer_pos - response.rect.left_center();
                                        ui.ctx().translate_layer(layer_id, delta);
                                    }
                                }
                                false => {
                                    let response = render_entry(ui);
                                    *bottom = response.rect.bottom();

                                    // Check for drags:
                                    let response =
                                        ui.interact(response.rect, item_id, Sense::drag());
                                    if response.hovered() {
                                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                                    }
                                }
                            }
                        }
                        if let Some(item_for_deletion_idx) = item_for_deletion_idx {
                            self.items.remove(item_for_deletion_idx);
                            self.bottoms.pop();
                        }
                        ui.shrink_width_to_current();
                        ui.vertical_centered(|ui| {
                            if let Some(create_item) = &self.create_item {
                                if !default_item_present {
                                    ui.separator();
                                    if ui.button("+").clicked() {
                                        self.items.push((create_item)());
                                        self.bottoms.push(0.0);
                                    }
                                }
                            }
                        });
                    }
                    false => {
                        for item in self.items.iter_mut() {
                            show_item(item, ui);
                        }
                    }
                }
            });

            let outer_rect =
                Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);

            let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());
            let is_anything_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());
            let style = if is_anything_being_dragged && response.hovered() {
                ui.visuals().widgets.active
            } else {
                ui.visuals().widgets.inactive
            };

            if self.active {
                ui.painter().set(
                    where_to_put_background,
                    RectShape {
                        rect,
                        rounding: style.rounding,
                        fill: ui.visuals().panel_fill,
                        stroke: style.bg_stroke,
                    },
                );
            }
        });
    }

    fn fix_dragged_item_position(&mut self, ui: &mut Ui) {
        let mut dragged_item: Option<usize> = None;
        for (idx, item) in self.items.iter().enumerate() {
            let item_id = Id::new(ID_SOURCE).with(item);
            if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                dragged_item = Some(idx);
            }
        }

        // move the dragged item in the list position relevant to its Y position
        if let Some(dragged_current_idx) = dragged_item {
            // there can't be a dragged item on the first frame, so this will never get
            // executed with bottoms of 0s
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                if self.active {
                    let dragged_new_idx = match self
                        .bottoms
                        .binary_search_by(|x| x.partial_cmp(&pointer_pos.y).unwrap())
                    {
                        Ok(dragged_new_idx) => dragged_new_idx,
                        Err(dragged_new_idx) => dragged_new_idx,
                    };

                    let dragged_new_idx = min(dragged_new_idx, self.items.len() - 1);
                    if dragged_new_idx != dragged_current_idx {
                        self.items.move_item(dragged_current_idx, dragged_new_idx)
                    }
                }
            }
        }
    }
}
