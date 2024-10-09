use egui::{Pos2, Rect};
use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableTable};

pub struct AutoScroll {
    pub scroll_offset: f32,
    pub enabled: bool,
    pub distance_from_min: f32,
    pub distance_from_max: f32,
    pub max_speed: f32,
}

impl Default for AutoScroll {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            enabled: false,
            distance_from_min: 200.0,
            distance_from_max: 120.0,
            max_speed: 30.0,
        }
    }
}

impl AutoScroll {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Default::default()
        }
    }
    #[must_use] pub const fn distance_from_min(mut self, distance: f32) -> Self {
        self.distance_from_min = distance;
        self
    }
    #[must_use] pub const fn distance_from_max(mut self, distance: f32) -> Self {
        self.distance_from_max = distance;
        self
    }

    #[must_use] pub const fn max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }

    pub(crate) fn start_scroll(&mut self, max_rect: Rect, pointer: Option<Pos2>) -> Option<f32> {
        if !self.enabled {
            return None;
        }

        if let Some(loc) = pointer {
            let pointer_y = loc.y;

            // Min gets a bit more space as the header is along the way
            let min_y = max_rect.min.y + self.distance_from_min;
            let max_y = max_rect.max.y - self.distance_from_max;

            // Check if the pointer is within the allowed Y range
            let within_y = pointer_y >= min_y && pointer_y <= max_y;

            // Whether the mouse is above the minimum y point
            let above_y = pointer_y < min_y;
            // Whether the mouse is below the maximum y point
            let below_y = pointer_y > max_y;

            let max_speed = self.max_speed;

            // Only scroll if the pointer is outside the allowed Y range
            if !within_y {
                let distance: f32;
                let direction: f32; // -1 for upwards, 1 for downwards

                if above_y {
                    // If above, calculate distance from min_y and scroll upwards
                    distance = (min_y - pointer_y).abs();
                    direction = -1.0; // Scroll up
                } else if below_y {
                    // If below, calculate distance from max_y and scroll downwards
                    distance = (pointer_y - max_y).abs();
                    direction = 1.0; // Scroll down
                } else {
                    return None;
                }

                // Scale the speed by distance, with a cap at max_speed
                let speed_factor = max_speed * (distance / 100.0).clamp(0.1, 1.0);

                self.scroll_offset += direction * speed_factor;

                // Ensure scroll offset doesn't go negative
                if self.scroll_offset < 0.0 {
                    self.scroll_offset = 0.0;
                }

                return Some(self.scroll_offset);
            }
        }
        None
    }
}

impl<Row, F, Conf> SelectableTable<Row, F, Conf>
where
    Row: Clone + Send,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F, Conf>
        + ColumnOrdering<Row>,
    Conf: Default,
{
    pub(crate) fn update_scroll_offset(&mut self, offset: f32) {
        self.auto_scroll.scroll_offset = offset;
    }

    #[must_use]
    pub const fn auto_scroll(mut self) -> Self {
        self.auto_scroll.enabled = true;
        self
    }

    #[must_use]
    pub const fn scroll_speed(mut self, speed: f32) -> Self {
        self.auto_scroll.max_speed = speed;
        self
    }

    #[must_use]
    pub const fn set_auto_scroll(mut self, scroll: AutoScroll) -> Self {
        self.auto_scroll = scroll;
        self
    }

    pub fn update_auto_scroll(&mut self, scroll: AutoScroll) {
        self.auto_scroll = scroll;
    }
}
