mod row_selection;

use egui::{Event, Key, Response, Sense, Ui};
use egui_extras::TableBuilder;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Default, Clone, Copy)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

pub trait ColumnOrdering<Row>
where
    Row: Clone + Send,
{
    /// Compares two rows for ordering based on the column.
    ///
    /// This method takes two row references and the desired sort order
    /// and returns an `Ordering` indicating their relative order.
    ///
    /// # Example Implementation:
    ///
    /// ```rust
    /// impl ColumnOrdering<MyRow> for ColumnName {
    ///     fn order_by(&self, row_1: &MyRow, row_2: &MyRow, sort_order: SortOrder) -> Ordering {
    ///         match self {
    ///             ColumnName::UserID => match sort_order {
    ///                 SortOrder::Ascending => row_1.user_id.cmp(&row_2.user_id),
    ///                 SortOrder::Descending => row_2.user_id.cmp(&row_1.user_id),
    ///             },
    ///             ColumnName::Name => match sort_order {
    ///                 SortOrder::Ascending => row_1.name.cmp(&row_2.name),
    ///                 SortOrder::Descending => row_2.name.cmp(&row_1.name),
    ///             },
    ///             ColumnName::Username => match sort_order {
    ///                 SortOrder::Ascending => row_1.username.cmp(&row_2.username),
    ///                 SortOrder::Descending => row_2.username.cmp(&row_1.username),
    ///             },
    ///         }
    ///     }
    /// }
    /// ```
    fn order_by(&self, row_1: &Row, row_2: &Row, sort_order: SortOrder) -> Ordering;
}

pub trait ColumnOperations<Row, F>
where
    Row: Clone + Send,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F>
        + ColumnOrdering<Row>,
{
    /// Create your header UI based on the column type. `sort_order` will be None if this column is
    /// not being used for sorting. Useful for adding emoji or icons ↑ ↓ to highlight ascending or
    /// descending sort. None for no header UI
    fn create_header(
        &self,
        ui: &mut Ui,
        sort_order: Option<SortOrder>,
        table: &mut SelectableTable<Row, F>,
    ) -> Option<Response>;
    /// Create your table row UI based on the column type and the row data
    fn create_table_row(
        &self,
        ui: &mut Ui,
        row: &Row,
        selected: bool,
        table: &mut SelectableTable<Row, F>,
    ) -> Response;

    fn column_text(&self, row: &Row) -> String;
}

#[derive(Clone)]
pub struct SelectableRow<Row, F>
where
    Row: Clone + Send,
    F: Eq + Hash + Clone + Ord + Send + Sync + Default,
{
    row_data: Row,
    id: i64,
    selected_columns: HashSet<F>,
}

pub struct SelectableTable<Row, F>
where
    Row: Clone + Send,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F>
        + ColumnOrdering<Row>,
{
    all_columns: Vec<F>,
    column_number: HashMap<F, usize>,
    rows: HashMap<i64, SelectableRow<Row, F>>,
    formatted_rows: Vec<SelectableRow<Row, F>>,
    sorted_by: F,
    sort_order: SortOrder,
    drag_started_on: Option<(i64, F)>,
    active_columns: HashSet<F>,
    active_rows: HashSet<i64>,
    last_active_row: Option<i64>,
    last_active_column: Option<F>,
    beyond_drag_point: bool,
    indexed_ids: HashMap<i64, usize>,
    scroll_offset: f32,
    last_id_used: i64,
    /// Set to auto call `recreate_rows` after `add_modify_row` has been called this many times.
    /// Useful when there are large number of rows adding/modification happening and want to reload
    /// the UI without keeping track of the count manually
    reload_after: Option<u32>,
    reload_count: u32,
}

impl<Row, F> SelectableTable<Row, F>
where
    Row: Clone + Send,
    F: Eq
        + Hash
        + Clone
        + Ord
        + Send
        + Sync
        + Default
        + ColumnOperations<Row, F>
        + ColumnOrdering<Row>,
{
    #[must_use]
    pub fn new(columns: Vec<F>) -> Self {
        let all_columns = columns.clone();
        let mut column_number = HashMap::new();

        for (index, col) in columns.into_iter().enumerate() {
            column_number.insert(col, index);
        }
        Self {
            all_columns,
            column_number,
            last_id_used: 0,
            rows: HashMap::new(),
            formatted_rows: Vec::new(),
            sorted_by: F::default(),
            sort_order: SortOrder::default(),
            drag_started_on: None,
            active_columns: HashSet::new(),
            active_rows: HashSet::new(),
            last_active_row: None,
            last_active_column: None,
            beyond_drag_point: false,
            indexed_ids: HashMap::new(),
            scroll_offset: 0.0,
            reload_after: None,
            reload_count: 0,
        }
    }

    #[must_use]
    pub fn set_auto_reload(mut self, count: u32) -> Self {
        self.reload_after = Some(count);
        self
    }
    /// Show Ui. The column list must be given in the correct order, from the first column to the
    /// last column.
    pub fn show_ui<Fn>(&mut self, ui: &mut Ui, table_builder: Fn)
    where
        Fn: FnOnce(TableBuilder) -> TableBuilder,
    {
        let is_ctrl_pressed = ui.ctx().input(|i| i.modifiers.ctrl);
        let key_a_pressed = ui.ctx().input(|i| i.key_pressed(Key::A));
        let copy_initiated = ui.ctx().input(|i| i.events.contains(&Event::Copy));
        let ctx = ui.ctx().clone();

        if copy_initiated {
            self.copy_selected_cells(ui);
        }
        if is_ctrl_pressed && key_a_pressed {
            self.select_all();
        }

        let pointer_location = ui.input(|i| i.pointer.hover_pos());
        let max_rec = ui.max_rect();

        let mut table = TableBuilder::new(ui);

        table = table_builder(table);

        if self.drag_started_on.is_some() {
            if let Some(loc) = pointer_location {
                let pointer_y = loc.y;

                // Min gets a bit more space as the header is along the way
                let min_y = max_rec.min.y + 200.0;
                let max_y = max_rec.max.y - 120.0;

                // Whether the mouse is within the space where the vertical scrolling should not happen
                let within_y = pointer_y >= min_y && pointer_y <= max_y;

                // Whether the mouse is above the minimum y point
                let above_y = pointer_y < min_y;
                // Whether the mouse is below the maximum y point
                let below_y = pointer_y > max_y;

                let max_distance = 100.0;
                let max_speed = 30.0;

                if !within_y {
                    let speed_factor: f32;

                    if above_y {
                        let distance = (min_y - pointer_y).abs();
                        speed_factor = max_speed * (distance / max_distance).clamp(0.1, 1.0);

                        self.scroll_offset -= speed_factor;
                        if self.scroll_offset < 0.0 {
                            self.scroll_offset = 0.0;
                        }
                    } else if below_y {
                        let distance = (pointer_y - max_y).abs();
                        speed_factor = max_speed * (distance / max_distance).clamp(0.1, 1.0);

                        self.scroll_offset += speed_factor;
                    }

                    table = table.vertical_scroll_offset(self.scroll_offset);
                    ctx.request_repaint();
                }
            }
        };
        let output = table
            .header(20.0, |mut header| {
                for column_name in &self.all_columns.clone() {
                    header.col(|ui| {
                        let mut sort_order = None;

                        if &self.sorted_by == column_name {
                            sort_order = Some(self.sort_order);
                        }

                        let Some(resp) = column_name.create_header(ui, sort_order, self) else {
                            return;
                        };

                        if resp.clicked() {
                            let is_selected = &self.sorted_by == column_name;
                            if is_selected {
                                self.change_sort_order();
                            } else {
                                self.change_sorted_by(column_name);
                            }
                            self.recreate_rows();
                        }
                    });
                }
            })
            .body(|body| {
                let table_rows = self.rows();
                body.rows(25.0, table_rows.len(), |mut row| {
                    let index = row.index();
                    let row_data = &table_rows[index];
                    // TODO: Maybe allow auto creating row number column if true?
                    //
                    // row.col(|ui| {
                    //     ui.add_sized(ui.available_size(), Label::new(format!("{}", index + 1)));
                    // });
                    for column_name in &self.all_columns.clone() {
                        row.col(|ui| {
                            let selected = row_data.selected_columns.contains(column_name);
                            let mut resp = column_name.create_table_row(
                                ui,
                                &row_data.row_data,
                                selected,
                                self,
                            );

                            resp = resp.interact(Sense::drag());

                            if resp.drag_started() {
                                // If CTRL is not pressed down and the mouse right click is not pressed, unselect all cells
                                // Right click for context menu
                                if !ui.ctx().input(|i| i.modifiers.ctrl)
                                    && !ui.ctx().input(|i| i.pointer.secondary_clicked())
                                {
                                    self.unselect_all();
                                }
                                self.drag_started_on = Some((row_data.id, column_name.clone()));
                            }

                            let pointer_released = ui.input(|a| a.pointer.primary_released());

                            if pointer_released {
                                self.last_active_row = None;
                                self.last_active_column = None;
                                self.drag_started_on = None;
                                self.beyond_drag_point = false;
                            }

                            if resp.clicked() {
                                // If CTRL is not pressed down and the mouse right click is not pressed, unselect all cells
                                if !ui.ctx().input(|i| i.modifiers.ctrl)
                                    && !ui.ctx().input(|i| i.pointer.secondary_clicked())
                                {
                                    self.unselect_all();
                                }
                                self.select_single_row_cell(row_data.id, column_name);
                            }

                            if ui.ui_contains_pointer() && self.drag_started_on.is_some() {
                                if let Some(drag_start) = self.drag_started_on.as_ref() {
                                    // Only call drag either when not on the starting drag row/column or went beyond the
                                    // drag point at least once. Otherwise normal click would be considered as drag
                                    if drag_start.0 != row_data.id
                                        || &drag_start.1 != column_name
                                        || self.beyond_drag_point
                                    {
                                        let is_ctrl_pressed = ui.ctx().input(|i| i.modifiers.ctrl);
                                        self.select_dragged_row_cell(
                                            row_data.id,
                                            column_name,
                                            is_ctrl_pressed,
                                        );
                                    }
                                }
                            }
                        });
                    }
                });
            });
        let scroll_offset = output.state.offset.y;
        self.scroll_offset = scroll_offset;
    }

    /// Add or modify existing rows as necessary. Must call `recreate_rows` for any modifications
    /// to show up in the UI.
    pub fn add_modify_row<Fn>(&mut self, table: Fn)
    where
        Fn: FnOnce(
            &mut HashMap<i64, SelectableRow<Row, F>>,
            &Vec<SelectableRow<Row, F>>,
            &HashMap<i64, usize>,
        ) -> Option<Row>,
    {
        let new_row = table(&mut self.rows, &self.formatted_rows, &self.indexed_ids);

        if let Some(row) = new_row {
            let selected_columns: HashSet<F> = HashSet::new();
            let new_row = SelectableRow {
                row_data: row,
                id: self.last_id_used,
                selected_columns,
            };
            self.rows.insert(new_row.id, new_row);
            self.last_id_used += 1;
        }

        if let Some(count) = self.reload_after {
            self.reload_count += 1;

            if self.reload_count >= count {
                self.recreate_rows();
                self.reload_count = 0;
            }
        }
    }

    /// Called the beginning when creating the Table UI. Ensures that `formatted_rows` is never
    /// empty
    fn rows(&mut self) -> Vec<SelectableRow<Row, F>> {
        if self.formatted_rows.len() != self.rows.len() {
            self.sort_rows();
        }
        self.formatted_rows.clone()
    }

    /// Sort the rows to the current sorting order and column and save them for later reuse
    fn sort_rows(&mut self) {
        let mut row_data: Vec<SelectableRow<Row, F>> = self.rows.clone().into_values().collect();

        row_data.par_sort_by(|a, b| {
            self.sorted_by
                .order_by(&a.row_data, &b.row_data, self.sort_order)
        });

        let indexed_data = row_data
            .iter()
            .enumerate()
            .map(|(index, row)| (row.id, index))
            .collect();

        self.indexed_ids = indexed_data;

        self.formatted_rows = row_data;
    }

    fn change_sort_order(&mut self) {
        self.unselect_all();
        if let SortOrder::Ascending = self.sort_order {
            self.sort_order = SortOrder::Descending;
        } else {
            self.sort_order = SortOrder::Ascending;
        }
    }

    fn change_sorted_by(&mut self, sort_by: &F) {
        self.unselect_all();
        self.sorted_by = sort_by.clone();
        self.sort_order = SortOrder::default();
    }

    /// Recreate the rows that are being shown in the UI in the next frame load. Frequently calling
    /// this with a very large number of rows can cause performance issues.
    pub fn recreate_rows(&mut self) {
        self.formatted_rows.clear();
        self.active_rows.clear();
        self.active_columns.clear();
    }

    fn first_column(&self) -> F {
        self.all_columns[0].clone()
    }

    fn last_column(&self) -> F {
        self.all_columns[self.all_columns.len() - 1].clone()
    }

    fn column_to_num(&self, column: &F) -> usize {
        *self
            .column_number
            .get(column)
            .expect("Not in the column list")
    }

    fn next_column(&self, column: &F) -> F {
        let current_column_num = self.column_to_num(column);
        if current_column_num == self.all_columns.len() - 1 {
            self.all_columns[0].clone()
        } else {
            self.all_columns[current_column_num + 1].clone()
        }
    }

    fn previous_column(&self, column: &F) -> F {
        let current_column_num = self.column_to_num(column);
        if current_column_num == 0 {
            self.all_columns[self.all_columns.len() - 1].clone()
        } else {
            self.all_columns[current_column_num - 1].clone()
        }
    }
}
