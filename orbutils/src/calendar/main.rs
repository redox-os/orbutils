extern crate orbtk;
extern crate chrono;
extern crate orbclient;
extern crate redox_log;

use orbtk::{Rect, Window, WindowBuilder, Grid, Label, Button, Style};
use orbtk::traits::{Place, Text, Click};
use orbtk::theme::Theme;
use chrono::prelude::*;
use chrono::Duration;
use std::ops::{Sub, Add};
use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver};
use redox_log::{OutputBuilder, RedoxLogger};

static CALENDAR_THEME_CSS: &'static str = include_str!("theme.css");

pub struct TimeMachine {
    date: DateTime<Local>,
}

impl TimeMachine {

    pub fn new() -> Self {
        TimeMachine {
            date: Local::now(),
        }
    }

    pub fn increase_month(&mut self) {
        self.date = self.date.add(Duration::days(31))
                        .with_day(1)
                        .unwrap();
    }

    pub fn decrease_month(&mut self) {
        self.date = self.date.sub(Duration::days(28))
                        .with_day(1)
                        .unwrap();
    }

    pub fn get_date(&mut self) -> DateTime<Local> {
        self.date
    }
}

enum CalendarCommand {
    IncreaseMonth(),
    DecreaseMonth(),
}

pub struct Calendar {
    window: Window,
    window_width: u32,
    cell_width: u32,
    cell_height: u32,
    cell_day_name_height: u32,
    date: TimeMachine,
    grid_calendar: Arc<Grid>,
    label_date: Arc<Label>,
    rx: Receiver<CalendarCommand>,
}

impl Calendar {

    pub fn new() -> Self {
        let (tx, rx) = channel();
        let cell_width = 90;
        let cell_height = 90;
        let cell_day_name_height = 16;
        let window_width = 7 * (cell_width + 8) + 8;
        let window_height = 6 * (cell_height + 8) + 16 + cell_day_name_height + 24;
        let theme = Theme::parse(CALENDAR_THEME_CSS);

        let mut window_builder = WindowBuilder::new(Rect::new(-1, -1, window_width, window_height), "Calendar");
        window_builder = window_builder.theme(theme);
        let window = window_builder.build();

        let label_date = Label::new();
        label_date.size(300, 16)
            .position((window_width / 2) as i32, 8);
        window.add(&label_date);

        {
            let offset = window_width / 2 - 100;
            let prev_month = Button::new();
            let tx = tx.clone();

            prev_month.text("<")
                .position(offset as i32, 6)
                .text_offset(5, 3)
                .size(20, 20)
                .on_click(move |_, _| {
                    tx.send(CalendarCommand::DecreaseMonth()).unwrap();
                });
            window.add(&prev_month);
        }

        {
            let next_month = Button::new();
            let offset = window_width / 2 + 80;
            let tx = tx.clone();
            next_month.text(">")
                .position(offset as i32, 6)
                .text_offset(5, 3)
                .size(20, 20)
                .on_click(move |_, _| {
                    tx.send(CalendarCommand::IncreaseMonth()).unwrap();
                });

            window.add(&next_month);
        }

        let grid = Grid::new();
        grid.position(8, 8 + 16 + 8)
            .spacing(8, 8);
        window.add(&grid);

        Calendar {
            window,
            window_width,
            cell_height,
            cell_width,
            cell_day_name_height,
            date: TimeMachine::new(),
            grid_calendar: grid,
            label_date: label_date.clone(),
            rx,
        }
    }

    pub fn redraw(&mut self) {
        self.redraw_date();
        self.redraw_grid();
    }

    pub fn redraw_grid(&mut self) {
        let date = self.date.get_date();
        self.grid_calendar.clear();
        let day_names = &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

        for (i, day) in day_names.iter().enumerate() {
            let label = Label::new();
            label.size(self.cell_width, self.cell_day_name_height)
                .text(*day)
                .text_offset((self.cell_width - day.len() as u32*8) as i32 /2, 0); 
            self.grid_calendar.insert(i, 0, &label);
        }

        let mut day = 1;
        let day_offset = date.with_day(1).unwrap().weekday().number_from_monday() as usize -1;
        for idx in day_offset..6*7usize {

            let d = date.with_day(day);
            match d {
                Some(x) => {
                    let cell = Label::new();
                    let text = format!("{}", x.day());
                    let text_offset = self.cell_width / 2 - (text.len() as u32 * 4);

                    cell
                        .size(self.cell_width, self.cell_height)
                        .with_class("date")
                        .text(text)
                        .text_offset(text_offset as i32, (self.cell_width / 2 -8) as i32);

                    if x.date_naive() == Local::now().date_naive() {
                        cell.with_class("today");
                    }

                    self.grid_calendar.insert(idx % 7, (idx / 7) + 1, &cell);
                },
                None => {}
            }
            day += 1;
        }

    }

    pub fn redraw_date(&mut self) {    
        let string_date = self.date.get_date().format("%B %Y").to_string();
        let label_position = self.window_width / 2 - (string_date.len() * 8 /2) as u32;
        self.label_date.text(string_date)
            .position(label_position as i32, 8);
    }

    pub fn exec(&mut self) {
        self.redraw();
        self.window.draw_if_needed();

        while self.window.running.get() {
            self.window.step();

            while let Ok(event) = self.rx.try_recv() {
                match event {
                    CalendarCommand::IncreaseMonth() => {
                        self.date.increase_month();
                        self.redraw();
                    },
                    CalendarCommand::DecreaseMonth() => {
                        self.date.decrease_month();
                        self.redraw();
                    }
                }
            }
            self.window.draw_if_needed();
        }
    }

}

fn main(){
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("calendar".into())
        .enable();

    Calendar::new().exec();
}
