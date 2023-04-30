use std::cmp::min;

use eframe::epaint::RectShape;
use egui::{CursorIcon, Id, LayerId, Order, Rect, Sense, Shape, Ui, Vec2};

pub(crate) trait ListUniqueID {
    fn id(&self) -> String;
}

pub(crate) struct DragAndDrop<T: Default + ListUniqueID + Clone> {
    active: bool,
    items: Vec<T>,
    bottoms: Vec<f32>,
    default_value_id: Id,
}
const ID_SOURCE: &str = "drag_and_drop";

impl<T: Default + ListUniqueID + Clone> DragAndDrop<T> {
    // transfering ownership of the Vec to the DragAndDrop
    pub(crate) fn new(items: Vec<T>) -> Self {
        let items_len = items.len();
        Self {
            active: false,
            items,
            bottoms: vec![0.0; items_len],
            default_value_id: Id::new(ID_SOURCE).with(T::default().id()),
        }
    }
    pub(crate) fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub(crate) fn show(&mut self, ui: &mut Ui, show_item: impl Fn(&mut T, &mut Ui)) {
        let margin = Vec2::splat(4.0);

        ui.vertical(|ui| {
            let outer_rect_bounds = ui.available_rect_before_wrap();
            let inner_rect = outer_rect_bounds.shrink2(margin);
            let where_to_put_background = ui.painter().add(Shape::Noop);

            let mut content_ui = ui.child_ui(inner_rect, *ui.layout());

            content_ui.vertical(|ui| {
                let mut dragged_item: Option<(usize, T)> = None;
                for (idx, item) in self.items.iter().enumerate() {
                    let item_id = Id::new(ID_SOURCE).with(item.id());
                    if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                        dragged_item = Some((idx, item.to_owned()));
                    }
                }

                if let Some((current_dragged_ix, dragged_item)) = dragged_item {
                    // there can't be a dragged item on the first frame, so this will never get
                    // executed with bottoms of 0s
                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                        if self.active {
                            match self
                                .bottoms
                                .binary_search_by(|x| x.partial_cmp(&pointer_pos.y).unwrap())
                            {
                                Ok(dragged_new_idx) => {
                                    let dragged_new_idx =
                                        min(dragged_new_idx, self.items.len() - 1);
                                    if dragged_new_idx != current_dragged_ix {
                                        self.items.remove(current_dragged_ix);
                                        self.items.insert(dragged_new_idx, dragged_item.to_owned());
                                    }
                                }
                                Err(dragged_new_idx) => {
                                    let dragged_new_idx =
                                        min(dragged_new_idx, self.items.len() - 1);
                                    if dragged_new_idx != current_dragged_ix {
                                        self.items.remove(current_dragged_ix);
                                        self.items.insert(dragged_new_idx, dragged_item.to_owned());
                                    }
                                }
                            }
                        }
                    }
                }
                let mut default_item_present = false;
                let mut item_for_deletion_idx = None;
                for (idx, (item, bottom)) in self
                    .items
                    .iter_mut()
                    .zip(self.bottoms.iter_mut())
                    .enumerate()
                {
                    if !self.active {
                        show_item(item, ui);
                        continue;
                    }
                    let item_id = Id::new(ID_SOURCE).with(item.id());
                    if item_id == self.default_value_id {
                        default_item_present = true;
                    }

                    let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(item_id));

                    if !is_being_dragged {
                        let response = ui
                            .horizontal(|ui| {
                                let scoped_handle = ui
                                    .scope(|ui| {
                                        ui.label(egui::RichText::new("::").heading().monospace())
                                    })
                                    .response;
                                show_item(item, ui);
                                if ui.small_button("-").clicked() {
                                    item_for_deletion_idx = Some(idx);
                                }
                                scoped_handle
                            })
                            .inner;
                        *bottom = response.rect.bottom();

                        // Check for drags:
                        let response = ui.interact(response.rect, item_id, Sense::drag());
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::Grab);
                        }
                    } else {
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

                        // Paint the body to a new layer:
                        let layer_id = LayerId::new(Order::Tooltip, item_id);
                        let response = ui
                            .with_layer_id(layer_id, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("::");
                                    show_item(item, ui);
                                })
                            })
                            .response;
                        *bottom = response.rect.bottom();

                        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                            let delta = pointer_pos - response.rect.left_center();
                            ui.ctx().translate_layer(layer_id, delta);
                        }
                    }
                }
                if let Some(item_for_deletion_idx) = item_for_deletion_idx {
                    self.items.remove(item_for_deletion_idx);
                    self.bottoms.pop();
                }
                if self.active {
                    ui.shrink_width_to_current();
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if !default_item_present {
                            if ui.button("+").clicked() {
                                self.items.push(T::default());
                                self.bottoms.push(0.0);
                            }
                        }
                    });
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
}
