use egui::Ui;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::{ColumnOperations, ColumnOrdering, SelectableTable};

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
    pub(crate) fn select_single_row_cell(&mut self, id: i64, column_name: &F) {
        self.active_columns.insert(column_name.clone());
        self.active_rows.insert(id);

        let target_index = self.indexed_ids.get(&id).unwrap();

        self.formatted_rows
            .get_mut(*target_index)
            .unwrap()
            .selected_columns
            .insert(column_name.clone());
    }
    pub(crate) fn select_dragged_row_cell(
        &mut self,
        id: i64,
        column_name: &F,
        is_ctrl_pressed: bool,
    ) {
        // If both same then the mouse is still on the same column on the same row so nothing to process
        if self.last_active_row == Some(id) && self.last_active_column == Some(column_name.clone())
        {
            return;
        }

        self.active_columns.insert(column_name.clone());
        self.beyond_drag_point = true;

        let drag_start = self.drag_started_on.clone().unwrap();

        // number of the column of drag starting point and the current cell that we are trying to select
        let drag_start_num = self.column_to_num(&drag_start.1);
        let ongoing_column_num = self.column_to_num(column_name);

        let mut new_column_set = HashSet::new();

        let get_previous = ongoing_column_num > drag_start_num;
        let mut ongoing_val = Some(drag_start.1.clone());
        // let mut ongoing_val = Some(ColumnName::from_num(drag_start_num));

        // row1: column(drag started here) column column
        // row2: column                    column column
        // row3: column                    column column
        // row4: column                    column column (currently here)
        //
        // The goal of this is to ensure from the drag starting point to all the columns till the currently here
        // are considered selected and the rest are removed from active selection even if it was considered active
        //
        // During fast mouse movement active rows can contain columns that are not in the range we are targeting
        // We go from one point to the other point and ensure except those columns nothing else is selected
        //
        // No active row removal if ctrl is being pressed!
        if is_ctrl_pressed {
            self.active_columns.insert(column_name.clone());
        } else if ongoing_column_num == drag_start_num {
            // new_column_set.insert(ColumnName::from_num(drag_start_num));
            new_column_set.insert(drag_start.1.clone());
            self.active_columns = new_column_set;
        } else {
            while ongoing_val.is_some() {
                let col = ongoing_val.unwrap();

                let next_column = if get_previous {
                    self.next_column(&col)
                    // col.get_next()
                } else {
                    // col.get_previous()
                    self.previous_column(&col)
                };

                new_column_set.insert(col);

                // if next_column == ColumnName::from_num(ongoing_column_num) {
                if &next_column == column_name {
                    new_column_set.insert(next_column);
                    ongoing_val = None;
                } else {
                    ongoing_val = Some(next_column);
                }
            }
            self.active_columns = new_column_set;
        }

        let current_row_index = self.indexed_ids.get(&id).unwrap();
        // The row the mouse pointer is on
        let current_row = self.formatted_rows.get_mut(*current_row_index).unwrap();

        // If this row already selects the column that we are trying to select, it means the mouse
        // moved backwards from an active column to another active column.
        //
        // Row: column1 column2 (mouse is here) column3 column4
        //
        // In this case, if column 3 or 4 is also found in the active selection then
        // the mouse moved backwards
        let row_contains_column = current_row.selected_columns.contains(column_name);

        let mut no_checking = false;
        // If we have some data of the last row and column that the mouse was on, then try to unselect
        if row_contains_column
            && self.last_active_row.is_some()
            && self.last_active_column.is_some()
        {
            let last_active_column = self.last_active_column.clone().unwrap();

            // Remove the last column selection from the current row where the mouse is if
            // the previous row and the current one matches
            //
            // column column column
            // column column column
            // column column (mouse is currently here) column(mouse was here)
            //
            // We unselect the bottom right corner column
            if &last_active_column != column_name && self.last_active_row.unwrap() == id {
                current_row.selected_columns.remove(&last_active_column);
                self.active_columns.remove(&last_active_column);
            }

            // Get the last row where the mouse was
            let current_row_index = self
                .indexed_ids
                .get(&self.last_active_row.unwrap())
                .unwrap();
            let last_row = self.formatted_rows.get_mut(*current_row_index).unwrap();

            self.last_active_row = Some(id);

            // If on the same row as the last row, then unselect the column from all other select row
            if id == last_row.id {
                if &last_active_column != column_name {
                    self.last_active_column = Some(column_name.clone());
                }
            } else {
                no_checking = true;
                // Mouse went 1 row above or below. So just clear all selection from that previous row
                last_row.selected_columns.clear();
            }
        } else {
            // We are in a new row which we have not selected before
            self.active_rows.insert(current_row.id);
            self.last_active_row = Some(id);
            self.last_active_column = Some(column_name.clone());
            current_row
                .selected_columns
                .clone_from(&self.active_columns);
        }

        let current_row_index = self.indexed_ids.get(&id).unwrap().to_owned();

        // Get the row number where the drag started on
        let drag_start_index = self.indexed_ids.get(&drag_start.0).unwrap().to_owned();

        if no_checking {
            self.remove_row_selection(current_row_index, drag_start_index, is_ctrl_pressed);
        } else {
            // If drag started on row 1, currently on row 5, check from row 4 to 1 and select all columns
            // else go through all rows till a row without any selected column is found. Applied both by incrementing or decrementing index.
            // In case of fast mouse movement following drag started point mitigates the risk of some rows not getting selected
            self.check_row_selection(true, current_row_index, drag_start_index);
            self.check_row_selection(false, current_row_index, drag_start_index);
            self.remove_row_selection(current_row_index, drag_start_index, is_ctrl_pressed);
        }
    }

    fn check_row_selection(&mut self, check_previous: bool, index: usize, drag_start: usize) {
        if index == 0 && check_previous {
            return;
        }

        if index + 1 == self.formatted_rows.len() && !check_previous {
            return;
        }

        let index = if check_previous { index - 1 } else { index + 1 };

        let current_row = self.formatted_rows.get(index).unwrap();
        let mut unselected_row = current_row.selected_columns.is_empty();

        // if for example drag started on row 5 and ended on row 10 but missed drag on row 7
        // Mark the rows as selected till the drag start row is hit (if recursively going that way)
        if (check_previous && index >= drag_start) || (!check_previous && index <= drag_start) {
            unselected_row = false;
        }

        let target_row = self.formatted_rows.get_mut(index).unwrap();

        if !unselected_row {
            target_row.selected_columns.clone_from(&self.active_columns);
            self.active_rows.insert(target_row.id);

            if check_previous {
                if index != 0 {
                    self.check_row_selection(check_previous, index, drag_start);
                }
            } else if index + 1 != self.formatted_rows.len() {
                self.check_row_selection(check_previous, index, drag_start);
            }
        }
    }

    fn remove_row_selection(
        &mut self,
        current_index: usize,
        drag_start: usize,
        is_ctrl_pressed: bool,
    ) {
        let active_ids = self.active_rows.clone();
        for id in active_ids {
            let ongoing_index = self.indexed_ids.get(&id).unwrap().to_owned();
            let target_row = self.formatted_rows.get_mut(ongoing_index).unwrap();

            if current_index > drag_start {
                if ongoing_index >= drag_start && ongoing_index <= current_index {
                    target_row.selected_columns.clone_from(&self.active_columns);
                } else if !is_ctrl_pressed {
                    target_row.selected_columns = HashSet::new();
                    self.active_rows.remove(&target_row.id);
                }
            } else if ongoing_index <= drag_start && ongoing_index >= current_index {
                target_row.selected_columns.clone_from(&self.active_columns);
            } else if !is_ctrl_pressed {
                target_row.selected_columns = HashSet::new();
                self.active_rows.remove(&target_row.id);
            }
        }
    }

    pub fn unselect_all(&mut self) {
        for id in &self.active_rows {
            let id_index = self.indexed_ids.get(id).unwrap();
            let target_row = self.formatted_rows.get_mut(*id_index).unwrap();
            target_row.selected_columns.clear();
        }
        self.active_columns.clear();
        self.last_active_row = None;
        self.last_active_column = None;
        self.active_rows.clear();
    }
    pub fn select_all(&mut self) {
        let mut all_rows = Vec::new();

        for row in self.formatted_rows.iter_mut() {
            row.selected_columns.extend(self.all_columns.clone());
            all_rows.push(row.id);
        }

        self.active_columns.extend(self.all_columns.clone());
        self.active_rows.extend(all_rows);
        self.last_active_row = None;
        self.last_active_column = None;
    }

    pub fn copy_selected_cells(&mut self, ui: &mut Ui) {
        let mut selected_rows = Vec::new();

        let mut column_max_length = HashMap::new();

        // Iter through all the rows and find the rows that have at least one column as selected
        // Keep track of the biggest length of a value of a column
        // active rows cannot be used here because hashset does not maintain an order.
        // So itering will give the rows in a different order than what is shown in the ui
        for row in &self.formatted_rows {
            if row.selected_columns.is_empty() {
                continue;
            }

            for column in &self.active_columns {
                if row.selected_columns.contains(column) {
                    let column_text = column.column_text(&row.row_data);
                    let field_length = column_text.len();
                    let entry = column_max_length.entry(column).or_insert(0);
                    if field_length > *entry {
                        column_max_length.insert(column, field_length);
                    }
                }
            }
            selected_rows.push(row);
        }

        let mut to_copy = String::new();

        // Target is to ensure a fixed length after each column value of a row
        // If for example highest len is 10 but the current row's
        // column value is 5, we will add the column value and add 5 more space after that
        // to ensure alignment
        for row in selected_rows {
            let mut ongoing_column = self.first_column();
            let mut row_text = String::new();
            loop {
                if self.active_columns.contains(&ongoing_column)
                    && row.selected_columns.contains(&ongoing_column)
                {
                    let column_text = ongoing_column.column_text(&row.row_data);
                    row_text += &format!(
                        "{:<width$}",
                        column_text,
                        width = column_max_length[&ongoing_column] + 1
                    );
                } else if self.active_columns.contains(&ongoing_column)
                    && !row.selected_columns.contains(&ongoing_column)
                {
                    row_text += &format!(
                        "{:<width$}",
                        "",
                        width = column_max_length[&ongoing_column] + 1
                    );
                }
                if self.last_column() == ongoing_column {
                    break;
                }
                ongoing_column = self.next_column(&ongoing_column);
            }
            to_copy.push_str(&row_text);
            to_copy.push('\n');
        }

        ui.ctx().output_mut(|i| i.copied_text = to_copy);
    }
}
