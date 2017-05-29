use std::fmt;

use prettytable;
use chrono::Duration;

/// Helper for formatting durations.
pub struct HumanDuration(pub Duration);

impl<'a> fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        macro_rules! try_write {
            ($num:expr, $str:expr) => {
                if $num == 1 { return write!(f, "1 {}", $str); }
                else if $num > 1 { return write!(f, "{} {}s", $num, $str); }
            }
        }

        try_write!(self.0.num_hours(), "hour");
        try_write!(self.0.num_minutes(), "minute");
        try_write!(self.0.num_seconds(), "second");
        write!(f, "0 seconds")
    }
}

pub struct Table {
    title_row: Option<TableRow>,
    rows: Vec<TableRow>,
}

pub struct TableRow {
    cells: Vec<prettytable::cell::Cell>,
}

impl TableRow {
    pub fn new() -> TableRow {
        TableRow {
            cells: vec![],
        }
    }

    pub fn add<D: fmt::Display>(&mut self, text: D) -> &mut TableRow {
        self.cells.push(prettytable::cell::Cell::new(&text.to_string()));
        self
    }

    fn make_row(&self) -> prettytable::row::Row {
        let mut row = prettytable::row::Row::empty();
        for cell in &self.cells {
            row.add_cell(cell.clone());
        }
        row
    }
}

impl Table {
    pub fn new() -> Table {
        Table {
            title_row: None,
            rows: vec![],
        }
    }

    pub fn title_row(&mut self) -> &mut TableRow {
        if self.title_row.is_none() {
            self.title_row = Some(TableRow::new());
        }
        self.title_row.as_mut().unwrap()
    }

    pub fn add_row(&mut self) -> &mut TableRow {
        self.rows.push(TableRow::new());
        let idx = self.rows.len() - 1;
        &mut self.rows[idx]
    }

    pub fn is_empty(&self) -> bool {
        self.rows.len() == 0
    }

    pub fn print(&self) {
        if self.is_empty() {
            return;
        }
        let mut tbl = prettytable::Table::new();
        tbl.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        if let Some(ref title_row) = self.title_row {
            tbl.set_titles(title_row.make_row());
        }
        for row in &self.rows {
            tbl.add_row(row.make_row());
        }
        tbl.print_tty(false);
    }
}
