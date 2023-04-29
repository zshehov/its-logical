use std::cmp::min;

use eframe::epaint::RectShape;
use egui::{CursorIcon, Id, LayerId, Order, Rect, Sense, Shape, Ui, Vec2};

enum Centers {
    Unknown(Vec<f32>),
    Known(Vec<f32>),
}

pub(crate) struct DragAndDrop {
    pub(crate) items: Vec<String>,
    centers: Centers,
}

impl DragAndDrop {
    pub(crate) fn new(items: Vec<String>) -> Self {
        let items_len = items.len();
        Self {
            items,
            centers: Centers::Unknown(vec![0.0; items_len]),
        }
    }

    pub(crate) fn show(&mut self, ui: &mut Ui) {
        let id_source = "drag_and_drop";
        let margin = Vec2::splat(4.0);

        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(margin);
        let where_to_put_background = ui.painter().add(Shape::Noop);

        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());

        content_ui.vertical(|ui| {
            match &mut self.centers {
                Centers::Unknown(centers) => {
                    // first iteration is only to get familiar with the centers
                    // TODO: this would probably break if windows size is changed
                    for (idx, item) in self.items.iter().enumerate() {
                        let response = ui.scope(|ui| ui.label(item)).response;
                        centers[idx] = response.rect.center().y;
                    }
                    self.centers = Centers::Known(centers.to_owned());
                }
                Centers::Known(centers) => {
                    let mut dragged_item: Option<(usize, String)> = None;
                    for (idx, item) in self.items.iter().enumerate() {
                        let item_id = Id::new(id_source).with(item.to_owned());
                        if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                            dragged_item = Some((idx, item.to_owned()));
                        }
                    }

                    if let Some((current_dragged_ix, dragged_item)) = dragged_item {
                        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                            match centers
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
                    for item in &self.items {
                        let item_id = Id::new(id_source).with(item.to_owned());

                        let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(item_id));

                        if !is_being_dragged {
                            let response = ui.scope(|ui| ui.label(item)).response;

                            // Check for drags:
                            let response = ui.interact(response.rect, item_id, Sense::drag());
                            if response.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::Grab);
                            }
                        } else {
                            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

                            // Paint the body to a new layer:
                            let layer_id = LayerId::new(Order::Tooltip, item_id);
                            let response = ui.with_layer_id(layer_id, |ui| ui.label(item)).response;

                            // Now we move the visuals of the body to where the mouse is.
                            // Normally you need to decide a location for a widget first,
                            // because otherwise that widget cannot interact with the mouse.
                            // However, a dragged component cannot be interacted with anyway
                            // (anything with `Order::Tooltip` always gets an empty [`Response`])
                            // So this is fine!

                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                let delta = pointer_pos - response.rect.center();
                                ui.ctx().translate_layer(layer_id, delta);
                            }
                        }
                    }
                }
            }
        });

        let outer_rect =
            Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);

        let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());
        let style = if response.hovered() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

        ui.painter().set(
            where_to_put_background,
            RectShape {
                rect,
                rounding: style.rounding,
                fill: style.bg_fill,
                stroke: style.bg_stroke,
            },
        );
    }
}
