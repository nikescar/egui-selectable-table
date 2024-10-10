mod auto_reload;
mod auto_scroll;
mod row_selection;

use auto_reload::AutoReload;
pub use auto_scroll::AutoScroll;
use egui::{Event, Key, Response, Sense, Ui};
use egui_extras::{TableBuilder, TableRow};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Enum representing the possible sort orders for table columns.
#[derive(Default, Clone, Copy)]
pub enum SortOrder {
    /// Sorts in ascending order (e.g., A to Z or 1 to 10).
    #[default]
    Ascending,
    /// Sorts in descending order (e.g., Z to A or 10 to 1).
    Descending,
}

/// Trait for defining how to order rows based on a specific column.
///
/// This trait should be implemented by users to specify how rows should be
/// compared for sorting purposes. The implementation can vary depending on
/// the type of column. For instance, string comparisons or numeric comparisons
/// can be handled differently depending on the column.
///
/// # Example
/// Suppose you have a struct `MyRow` with fields like `user_id`, `name`, and `username`.
/// You could implement this trait for each column to specify how rows should be compared.
///
/// ```rust,ignore
/// impl ColumnOrdering<MyRow> for ColumnName {
///     fn order_by(&self, row_1: &MyRow, row_2: &MyRow) -> Ordering {
///         match self {
///             ColumnName::UserID => row_1.user_id.cmp(&row_2.user_id),
///             ColumnName::Name => row_1.name.cmp(&row_2.name),
///             ColumnName::Username => row_1.username.cmp(&row_2.username),
///         }
///     }
/// }
/// ```
pub trait ColumnOrdering<Row>
where
    Row: Clone + Send,
{
    /// Compare two rows and return the ordering result (`Ordering`).
    ///
    /// This function defines how to order two rows based on the specific column.
    /// It returns `Ordering::Less`, `Ordering::Equal`, or `Ordering::Greater`
    /// to indicate whether `row_1` should be placed before, after, or at the same
    /// position as `row_2` when sorting.
    ///
    /// # Arguments
    /// * `row_1` - The first row for comparison.
    /// * `row_2` - The second row for comparison.
    ///
    /// # Returns
    /// * `Ordering` - Indicates the relative order between the two rows.
    fn order_by(&self, row_1: &Row, row_2: &Row) -> Ordering;
}

/// Trait for defining column-specific operations in a table UI.
///
/// This trait allows users to define how each column should behave within a table.
/// This includes how headers should be displayed, how each row in the table should be rendered,
/// and how to extract column-specific text for display purposes.
///
/// # Type Parameters:
/// * `Row` - The type representing each row in the table.
/// * `F` - A type that identifies columns, usually an enum or a field type.
/// * `Conf` - Configuration type for the table, useful for passing additional settings.
///
/// # Requirements:
/// You must implement this trait to specify the behavior of each column within
/// the context of your table UI.
pub trait ColumnOperations<Row, F, Conf>
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
    /// Create the header UI for this column.
    ///
    /// This function is responsible for creating the visual representation of the column header.
    /// The `sort_order` argument indicates whether the column is sorted and, if so, in which
    /// direction (ascending or descending). You can customize the header appearance based on
    /// this information, for example by adding icons or text.
    ///
    /// # Arguments
    /// * `ui` - A mutable reference to the UI context.
    /// * `sort_order` - An optional `SortOrder` representing the current sort state of the column.
    /// * `table` - A mutable reference to the `SelectableTable`, allowing you to interact with the table state.
    ///
    /// # Returns
    /// * `Option<Response>` - An optional response representing interaction with the UI.
    fn create_header(
        &self,
        ui: &mut Ui,
        sort_order: Option<SortOrder>,
        table: &mut SelectableTable<Row, F, Conf>,
    ) -> Option<Response>;

    /// Create the UI for a specific row in this column.
    ///
    /// This function is responsible for rendering the content of this column for a given row.
    /// It should handle user interactions like clicking or selection as necessary.
    ///
    /// # Arguments
    /// * `ui` - A mutable reference to the UI context.
    /// * `row` - A reference to the current `SelectableRow` for this table.
    /// * `column_selected` - A boolean indicating whether this column is selected.
    /// * `table` - A mutable reference to the `SelectableTable`, allowing interaction with the table state.
    ///
    /// # Returns
    /// * `Response` - The result of the UI interaction for this row.
    fn create_table_row(
        &self,
        ui: &mut Ui,
        row: &SelectableRow<Row, F>,
        column_selected: bool,
        table: &mut SelectableTable<Row, F, Conf>,
    ) -> Response;

    /// Extract the text representation of the column for the given row.
    ///
    /// This function should return the appropriate text representation of this column
    /// for the given row. It can be used to display the data in a simplified form, such
    /// as for debugging or plain text rendering.
    ///
    /// # Arguments
    /// * `row` - A reference to the row from which to extract the column text.
    ///
    /// # Returns
    /// * `String` - The text representation of this column for the row.
    fn column_text(&self, row: &Row) -> String;
}

/// Represents a row in a table with selectable columns.
///
/// This struct is used to store the data of a row along with its unique identifier (`id`)
/// and the set of selected columns for this row.
///
/// # Type Parameters:
/// * `Row` - The type representing the data stored in each row.
/// * `F` - The type used to identify each column, typically an enum or a type with unique values.
///
/// # Fields:
/// * `row_data` - The actual data stored in the row.
/// * `id` - A unique identifier for the row.
/// * `selected_columns` - A set of columns that are selected in this row.
#[derive(Clone)]
pub struct SelectableRow<Row, F>
where
    Row: Clone + Send,
    F: Eq + Hash + Clone + Ord + Send + Sync + Default,
{
    pub row_data: Row,
    pub id: i64,
    pub selected_columns: HashSet<F>,
}

/// A table structure that hold data for performing selection on drag, sorting, and displaying rows and more.
///
/// # Type Parameters
/// * `Row` - The type representing each row in the table.
/// * `F` - A type used to identify columns, often an enum or field type.
/// * `Conf` - Configuration type for additional table settings. This is made available anytime
///    when creating or modifying rows
pub struct SelectableTable<Row, F, Conf>
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
    /// List of all columns available in the table.
    all_columns: Vec<F>,
    /// Maps each column to its index in the table for quick lookup.
    column_number: HashMap<F, usize>,
    /// Stores all rows in the table, keyed by their unique ID.
    rows: HashMap<i64, SelectableRow<Row, F>>,
    /// The current set of formatted rows for display.
    formatted_rows: Vec<SelectableRow<Row, F>>,
    /// The column currently being used to sort the table.
    sorted_by: F,
    /// The current sort order (ascending or descending).
    sort_order: SortOrder,
    /// Tracks where a drag operation started in the table, if any.
    drag_started_on: Option<(i64, F)>,
    /// The columns that have at least 1 row with the column as selected
    active_columns: HashSet<F>,
    /// The rows that have at least 1 column as selected
    active_rows: HashSet<i64>,
    /// The last row where the pointer was
    last_active_row: Option<i64>,
    /// The last column where the pointer was
    last_active_column: Option<F>,
    /// Whether the pointer moved from the dragged point at least once
    beyond_drag_point: bool,
    /// Map of the row IDs to the indices of `formatted_rows`
    indexed_ids: HashMap<i64, usize>,
    /// The last ID that was used for a new row in the table.
    last_id_used: i64,
    /// Handles auto scroll operation when dragging
    auto_scroll: AutoScroll,
    /// Handles auto recreating the displayed rows with the latest data
    auto_reload: AutoReload,
    /// Whether to select the entire row when dragging and selecting instead of a single cell
    select_full_row: bool,
    /// Additional Parameters passed by you, available when creating new rows or header. Can
    /// contain anything implementing the `Default` trait
    pub config: Conf,
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
            auto_scroll: AutoScroll::default(),
            auto_reload: AutoReload::default(),
            select_full_row: false,
            config: Conf::default(),
        }
    }

    pub fn set_config(&mut self, conf: Conf) {
        self.config = conf;
    }

    #[must_use]
    pub fn config(mut self, conf: Conf) -> Self {
        self.config = conf;
        self
    }

    pub fn clear_all_rows(&mut self) {
        self.rows.clear();
        self.formatted_rows.clear();
        self.active_rows.clear();
        self.active_columns.clear();
    }

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

        let pointer = ui.input(|i| i.pointer.hover_pos());
        let max_rect = ui.max_rect();

        let mut table = TableBuilder::new(ui);

        table = table_builder(table);

        if self.drag_started_on.is_some() {
            if let Some(offset) = self.auto_scroll.start_scroll(max_rect, pointer) {
                table = table.vertical_scroll_offset(offset);
                ctx.request_repaint();
            }
        };
        let output = table
            .header(20.0, |mut header| {
                for column_name in &self.all_columns.clone() {
                    header.col(|ui| {
                        let sort_order = if &self.sorted_by == column_name {
                            Some(self.sort_order)
                        } else {
                            None
                        };

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
                body.rows(25.0, table_rows.len(), |row| {
                    let index = row.index();
                    let row_data = &table_rows[index];
                    self.handle_table_body(row, row_data);

                    // TODO: Maybe allow auto creating row number column if true?
                    //
                    // row.col(|ui| {
                    //     ui.add_sized(ui.available_size(), Label::new(format!("{}", index + 1)));
                    // });
                });
            });
        let scroll_offset = output.state.offset.y;
        self.update_scroll_offset(scroll_offset);
    }

    /// Add or modify existing rows as necessary. Must call `recreate_rows` for any modifications
    /// to show up in the UI. Use `auto_reload` to auto recreate rows after X amount of
    /// modifications.
    pub fn add_modify_row<Fn>(&mut self, table: Fn)
    where
        Fn: FnOnce(&mut HashMap<i64, SelectableRow<Row, F>>) -> Option<Row>,
    {
        let new_row = table(&mut self.rows);

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

        let reload = self.auto_reload.increment_count();

        if reload {
            self.recreate_rows();
        }
    }

    /// Only modify the rows that are being shown in the UI. Data will be lost if rows are
    /// recreated and not added/modified via `add_modify_rows`
    /// Does not count toward auto reload
    /// This should not be used if rows are being recreated actively
    pub fn add_modify_shown_row<Fn>(&mut self, table: Fn)
    where
        Fn: FnOnce(&mut Vec<SelectableRow<Row, F>>, &HashMap<i64, usize>),
    {
        table(&mut self.formatted_rows, &self.indexed_ids);
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
            let ordering = self.sorted_by.order_by(&a.row_data, &b.row_data);
            match self.sort_order {
                SortOrder::Ascending => ordering,
                SortOrder::Descending => ordering.reverse(),
            }
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
        if matches!(self.sort_order, SortOrder::Ascending) {
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

    fn handle_table_body(&mut self, mut row: TableRow, row_data: &SelectableRow<Row, F>) {
        for column_name in &self.all_columns.clone() {
            row.col(|ui| {
                let selected = row_data.selected_columns.contains(column_name);
                let mut resp = column_name.create_table_row(ui, row_data, selected, self);

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
                            self.select_dragged_row_cell(row_data.id, column_name, is_ctrl_pressed);
                        }
                    }
                }
            });
        }
    }
}
