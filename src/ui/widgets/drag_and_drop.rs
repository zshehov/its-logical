use std::cmp::min;

use eframe::epaint::RectShape;
use egui::{CursorIcon, Id, LayerId, Order, Rect, Sense, Shape, Ui, Vec2};

pub(crate) struct DragAndDrop {
    pub(crate) items: Vec<String>,
    bottoms: Vec<f32>,
}

impl DragAndDrop {
    pub(crate) fn new(items: Vec<String>) -> Self {
        let items_len = items.len();
        Self {
            items,
            bottoms: vec![0.0; items_len],
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
            let mut dragged_item: Option<(usize, String)> = None;
            for (idx, item) in self.items.iter().enumerate() {
                let item_id = Id::new(id_source).with(item.to_owned());
                if ui.memory(|mem| mem.is_being_dragged(item_id)) {
                    dragged_item = Some((idx, item.to_owned()));
                }
            }

            if let Some((current_dragged_ix, dragged_item)) = dragged_item {
                // there can't be a dragged item on the first frame, so this will never get
                // executed with bottoms of 0s
                if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                    match self
                        .bottoms
                        .binary_search_by(|x| x.partial_cmp(&pointer_pos.y).unwrap())
                    {
                        Ok(dragged_new_idx) => {
                            let dragged_new_idx = min(dragged_new_idx, self.items.len() - 1);
                            if dragged_new_idx != current_dragged_ix {
                                self.items.remove(current_dragged_ix);
                                self.items.insert(dragged_new_idx, dragged_item.to_owned());
                            }
                        }
                        Err(dragged_new_idx) => {
                            let dragged_new_idx = min(dragged_new_idx, self.items.len() - 1);
                            if dragged_new_idx != current_dragged_ix {
                                self.items.remove(current_dragged_ix);
                                self.items.insert(dragged_new_idx, dragged_item.to_owned());
                            }
                        }
                    }
                }
            }
            for (item, bottom) in self.items.iter().zip(self.bottoms.iter_mut()) {
                let item_id = Id::new(id_source).with(item.to_owned());

                let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(item_id));

                if !is_being_dragged {
                    let response = ui.scope(|ui| ui.label(item)).response;
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
                    let response = ui.with_layer_id(layer_id, |ui| ui.label(item)).response;
                    *bottom = response.rect.bottom();

                    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                        let delta = pointer_pos - response.rect.center();
                        ui.ctx().translate_layer(layer_id, delta);
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
