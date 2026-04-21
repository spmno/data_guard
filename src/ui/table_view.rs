use gpui::{Color, View, ViewContext, vstack, hstack, label};

pub struct TableView {
    rows: Vec<Vec<String>>,
    columns: Vec<String>,
}

impl TableView {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            rows,
            columns,
        }
    }
}

impl View for TableView {
    fn ui_name() -> &'static str {
        "TableView"
    }

    fn render(&mut self, cx: &mut ViewContext<Self>) {
        let mut ui = cx.new_ui();
        ui.add(|cx| {
            vstack((
                // Header row
                hstack(self.columns.iter().map(|col| {
                    label(col).padding(8.0).background(Color::rgb(220, 220, 220))
                })).spacing(1.0),
                
                // Data rows
                vstack(self.rows.iter().map(|row| {
                    hstack(row.iter().map(|cell| {
                        let truncated = if cell.len() > 50 {
                            &cell[..50] + "..."
                        } else {
                            cell
                        };
                        label(truncated).padding(8.0).background(Color::rgb(255, 255, 255))
                    })).spacing(1.0)
                })).spacing(1.0),
            )).spacing(1.0)
        });
        ui.build();
    }
}
