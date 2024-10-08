use eframe::{App, CreationContext, Frame};
use egui::{
    global_theme_preference_switch, Align, Button, CentralPanel, Context, Layout, SelectableLabel,
    Slider, ThemePreference,
};
use egui_extras::Column;
use egui_selectable_table::{ColumnOperations, ColumnOrdering, SelectableTable, SortOrder};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub struct MainWindow {
    add_rows: bool,
    row_to_add: u64,
    row_num: u64,
    row_count: u64,
    table: SelectableTable<TableRow, TableColumns>,
}

impl MainWindow {
    pub fn new(cc: &CreationContext) -> Self {
        cc.egui_ctx
            .options_mut(|a| a.theme_preference = ThemePreference::Light);
        let all_columns = TableColumns::iter().collect();
        // Auto reload after each 10k table row add or modification
        let table = SelectableTable::new(all_columns).set_auto_reload(10000);
        MainWindow {
            add_rows: false,
            row_to_add: 0,
            row_num: 0,
            row_count: 0,
            table,
        }
    }
}

impl App for MainWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                global_theme_preference_switch(ui);
                ui.separator();
                ui.label("Total Rows to add:");
                ui.add(Slider::new(&mut self.row_count, 10000..=200000));

                let button_enabled = !self.add_rows;
                let button = ui.add_enabled(button_enabled, Button::new("Create Rows"));
                if button.clicked() {
                    self.add_rows = true;
                    self.row_to_add = self.row_count;

                    // Clearly previously added rows
                    self.table.add_modify_row(|t, _, _| {
                        t.clear();
                        None
                    });
                };
            });
            ui.separator();

            self.table.show_ui(ui, |table| {
                let mut table = table
                    .drag_to_scroll(false)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .drag_to_scroll(false)
                    .auto_shrink([false; 2])
                    .min_scrolled_height(0.0);

                for _col in TableColumns::iter() {
                    table = table.column(Column::initial(150.0))
                }
                table
            });

            if self.add_rows {
                for _num in 0..1000 {
                    self.table.add_modify_row(|_, _, _| {
                        let new_row = TableRow {
                            field_1: self.row_num,
                            field_2: self.row_num as i64 * 10,
                            field_3: format!("field 3 with row num: {}", self.row_num),
                            field_4: format!("field 4 with row num: {}", self.row_num),
                            field_5: format!("field 5 with row num: {}", self.row_num),
                            field_6: format!("field 6 with row num: {}", self.row_num),
                            field_7: format!("field 7 with row num: {}", self.row_num),
                        };
                        Some(new_row)
                    });
                    self.row_num += 1;
                    if self.row_num > self.row_to_add {
                        self.add_rows = false;
                        self.row_to_add = 0;
                        self.row_num = 0;
                        self.table.recreate_rows();
                        break;
                    }
                }
                ctx.request_repaint();
            }
        });
    }
}

#[derive(Clone, Default)]
struct TableRow {
    field_1: u64,
    field_2: i64,
    field_3: String,
    field_4: String,
    field_5: String,
    field_6: String,
    field_7: String,
}

#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Clone, Copy, Hash, Default, EnumIter)]
enum TableColumns {
    #[default]
    Field1,
    Field2,
    Field3,
    Field4,
    Field5,
    Field6,
    Field7,
}

impl ColumnOperations<TableRow, TableColumns> for TableColumns {
    fn column_text(&self, row: &TableRow) -> String {
        match self {
            TableColumns::Field1 => row.field_1.to_string(),
            TableColumns::Field2 => row.field_2.to_string(),
            TableColumns::Field3 => row.field_3.to_string(),
            TableColumns::Field4 => row.field_4.to_string(),
            TableColumns::Field5 => row.field_5.to_string(),
            TableColumns::Field6 => row.field_6.to_string(),
            TableColumns::Field7 => row.field_7.to_string(),
        }
    }
    fn create_header(
        &self,
        ui: &mut egui::Ui,
        sort_order: Option<egui_selectable_table::SortOrder>,
        _table: &mut SelectableTable<TableRow, TableColumns>,
    ) -> Option<egui::Response> {
        let mut text = match self {
            TableColumns::Field1 => "Field 1",
            TableColumns::Field2 => "Field 2",
            TableColumns::Field3 => "Field 3",
            TableColumns::Field4 => "Field 4",
            TableColumns::Field5 => "Field 5",
            TableColumns::Field6 => "Field 6",
            TableColumns::Field7 => "Field 7",
        }
        .to_string();
        if let Some(sort) = sort_order {
            match sort {
                SortOrder::Ascending => text += "🔽",
                SortOrder::Descending => text += "🔼",
            }
        }
        let selected = sort_order.is_some();
        let resp = ui.add_sized(ui.available_size(), SelectableLabel::new(selected, text));
        Some(resp)
    }
    fn create_table_row(
        &self,
        ui: &mut egui::Ui,
        row: &TableRow,
        selected: bool,
        _table: &mut SelectableTable<TableRow, TableColumns>,
    ) -> egui::Response {
        let text = match self {
            TableColumns::Field1 => row.field_1.to_string(),
            TableColumns::Field2 => row.field_2.to_string(),
            TableColumns::Field3 => row.field_3.to_string(),
            TableColumns::Field4 => row.field_4.to_string(),
            TableColumns::Field5 => row.field_5.to_string(),
            TableColumns::Field6 => row.field_6.to_string(),
            TableColumns::Field7 => row.field_7.to_string(),
        };
        ui.add_sized(ui.available_size(), SelectableLabel::new(selected, text))
    }
}

impl ColumnOrdering<TableRow> for TableColumns {
    fn order_by(
        &self,
        row_1: &TableRow,
        row_2: &TableRow,
        sort_order: SortOrder,
    ) -> std::cmp::Ordering {
        match self {
            TableColumns::Field1 => match sort_order {
                SortOrder::Ascending => row_1.field_1.cmp(&row_2.field_1),
                SortOrder::Descending => row_1.field_1.cmp(&row_2.field_1).reverse(),
            },
            TableColumns::Field2 => match sort_order {
                SortOrder::Ascending => row_1.field_2.cmp(&row_2.field_2),
                SortOrder::Descending => row_1.field_2.cmp(&row_2.field_2).reverse(),
            },
            TableColumns::Field3 => match sort_order {
                SortOrder::Ascending => row_1.field_3.cmp(&row_2.field_3),
                SortOrder::Descending => row_1.field_3.cmp(&row_2.field_3).reverse(),
            },
            TableColumns::Field4 => match sort_order {
                SortOrder::Ascending => row_1.field_4.cmp(&row_2.field_4),
                SortOrder::Descending => row_1.field_4.cmp(&row_2.field_4).reverse(),
            },
            TableColumns::Field5 => match sort_order {
                SortOrder::Ascending => row_1.field_5.cmp(&row_2.field_5),
                SortOrder::Descending => row_1.field_5.cmp(&row_2.field_5).reverse(),
            },
            TableColumns::Field6 => match sort_order {
                SortOrder::Ascending => row_1.field_6.cmp(&row_2.field_6),
                SortOrder::Descending => row_1.field_6.cmp(&row_2.field_6).reverse(),
            },
            TableColumns::Field7 => match sort_order {
                SortOrder::Ascending => row_1.field_7.cmp(&row_2.field_7),
                SortOrder::Descending => row_1.field_7.cmp(&row_2.field_7).reverse(),
            },
        }
    }
}
