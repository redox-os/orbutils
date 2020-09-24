extern crate chrono;

use chrono::prelude::*;
use chrono::Duration;
use orbtk::{prelude::*, theming::config::ThemeConfig};
use std::ops::{Add, Sub};

const CALENDAR_GRID: &str = "_CLNDR_GRD";
const HEADER_GRID: &str = "_HDR_GRD";
const CURRENT_DATE_LABEL: &str = "_CRRNT_DT_LBL_ID";
const CELL_WIDTH: u32 = 90;
const CELL_HEIGHT: u32 = 90;
const CELL_DAY_NAME_HEIGHT: u32 = 16;
const PADDING: u32 = 8;
const WINDOW_WIDTH: u32 = 7 * (CELL_WIDTH + PADDING) + PADDING;
const WINDOW_HEIGHT: u32 = 6 * (CELL_HEIGHT + PADDING) + 16 + CELL_DAY_NAME_HEIGHT + 24;
const DAYS_OF_WEEK: [&'static str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

enum CalendarCommand {
    IncreaseMonth,
    DecreaseMonth,
}

struct TimeMachine {
    date: DateTime<Local>,
}

impl TimeMachine {
    fn new() -> Self {
        TimeMachine { date: Local::now() }
    }

    fn increase_month(&mut self) {
        self.date = self.date.add(Duration::days(31)).with_day(1).unwrap();
    }

    fn decrease_month(&mut self) {
        self.date = self.date.sub(Duration::days(28)).with_day(1).unwrap();
    }

    fn get_date(&self) -> DateTime<Local> {
        self.date
    }
}

impl Default for TimeMachine {
    fn default() -> Self {
        TimeMachine::new()
    }
}

#[derive(Default, AsAny)]
struct CalendarState {
    command: Option<CalendarCommand>,
    date: TimeMachine,
    header_grid: Entity,
    calendar_grid: Entity,
    label_date: Entity,
}

impl CalendarState {
    fn command(&mut self, command: CalendarCommand) {
        self.command = Some(command);
    }

    fn redraw_date(&self, ctx: &mut Context) {
        let string_date = self.date.get_date().format("%B %Y").to_string();
        ctx.get_widget(self.label_date)
            .set::<String>("text", string_date);
    }

    fn redraw_grid(&self, ctx: &mut Context) {
        ctx.clear_children_of(self.calendar_grid);

        let date = self.date.get_date();
        let mut day = 1;
        let day_offset = self
            .date
            .get_date()
            .with_day(1)
            .unwrap()
            .weekday()
            .number_from_monday() as usize
            - 1;
        let today = Local::now().date();

        for idx in day_offset..6 * 7usize {
            match date.with_day(day) {
                Some(x) => {
                    let styles = {
                        match x.date() == today {
                            true => ("date_today", "text_block_today"),
                            false => ("date_cell", "text_block"),
                        }
                    };
                    let cell = get_cell(
                        idx,
                        format!("{}", x.day()),
                        styles,
                        &mut ctx.build_context(),
                    );
                    if x.date() == today {
                        ctx.get_widget(cell)
                            .set::<String>("name", "today".to_string());
                    }

                    ctx.append_child_entity_to(cell, self.calendar_grid);
                }
                None => {}
            }
            day += 1;
        }
    }
}

fn get_cell(index: usize, text: String, styles: (&str, &str), ctx: &mut BuildContext) -> Entity {
    Container::new()
        .style(styles.0)
        .attach(Grid::column(index % 7))
        .attach(Grid::row(index / 7))
        .child(
            TextBlock::new()
                .text(text)
                .style(styles.1)
                .h_align("center")
                .v_align("center")
                .build(ctx),
        )
        .build(ctx)
}

impl State for CalendarState {
    fn init(&mut self, _: &mut Registry, ctx: &mut Context) {
        self.label_date = ctx
            .entity_of_child(CURRENT_DATE_LABEL)
            .expect("Could not find entity of current date label!");
        self.calendar_grid = ctx
            .entity_of_child(CALENDAR_GRID)
            .expect("Could not find entity of calendar grid!");
        self.header_grid = ctx
            .entity_of_child(HEADER_GRID)
            .expect("Could not find entity of header grid!");
        self.redraw_date(ctx);

        for (i, day) in DAYS_OF_WEEK.iter().enumerate() {
            let label = generate_weekday_label(i, day);
            ctx.append_child_to(label, self.header_grid);
        }

        self.redraw_grid(ctx);
    }

    fn update(&mut self, _: &mut Registry, ctx: &mut Context) {
        if let Some(command) = &self.command {
            match command {
                CalendarCommand::DecreaseMonth => {
                    self.date.decrease_month();
                    self.redraw_date(ctx);
                    self.redraw_grid(ctx);
                }
                CalendarCommand::IncreaseMonth => {
                    self.date.increase_month();
                    self.redraw_date(ctx);
                    self.redraw_grid(ctx);
                }
            }

            self.command = None;
        }
    }
}

widget!(CalendarView<CalendarState>);

impl Template for CalendarView {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        self.name("CalendarView").child(
            Stack::new()
                .orientation("vertical")
                .spacing(32.0)
                .child(
                    Grid::new()
                        .id(HEADER_GRID)
                        .h_align("center")
                        .width(7 * 90 + PADDING)
                        .margin(8.0)
                        .rows(Rows::create().push("auto").push("auto").build())
                        .columns(Columns::create().repeat(90, 7).build())
                        .child(
                            Button::new()
                                .style("button_styled")
                                .attach(Grid::row(0))
                                .attach(Grid::column(2))
                                .h_align("center")
                                .v_align("center")
                                .icon(material_icons_font::MD_KEYBOARD_ARROW_LEFT)
                                .on_click(move |states, _| -> bool {
                                    states
                                        .get_mut::<CalendarState>(id)
                                        .command(CalendarCommand::DecreaseMonth);
                                    true
                                })
                                .build(ctx),
                        )
                        .child(
                            TextBlock::new()
                                .style("text_block")
                                .id(CURRENT_DATE_LABEL)
                                .attach(Grid::row(0))
                                .attach(Grid::column(3))
                                .h_align("center")
                                .size(300, 16)
                                .v_align("center")
                                .build(ctx),
                        )
                        .child(
                            Button::new()
                                .style("button_styled")
                                .attach(Grid::row(0))
                                .attach(Grid::column(4))
                                .h_align("center")
                                .v_align("center")
                                .icon(material_icons_font::MD_KEYBOARD_ARROW_RIGHT)
                                .width(32)
                                .height(32)
                                .on_click(move |states, _| -> bool {
                                    states
                                        .get_mut::<CalendarState>(id)
                                        .command(CalendarCommand::IncreaseMonth);
                                    true
                                })
                                .build(ctx),
                        )
                        .build(ctx),
                )
                .child(
                    Grid::new()
                        .id(CALENDAR_GRID)
                        .h_align("center")
                        .margin((4.0, 4.0))
                        .width(7 * CELL_WIDTH + PADDING)
                        .rows(Rows::create().repeat("auto", 6).build())
                        .columns(Columns::create().repeat(90.0, 7).build())
                        .build(ctx),
                )
                .build(ctx),
        )
    }
}

fn generate_weekday_label(column_index: usize, day: &str) -> TextBlock {
    TextBlock::new()
        .attach(Grid::row(1))
        .attach(Grid::column(column_index))
        .h_align("center")
        .style("text_block")
        .text(day)
}

static CALENDAR_THEME: &str = include_str!("theme.ron");

fn theme() -> Theme {
    Theme::from_config(ThemeConfig::from(CALENDAR_THEME))
}

fn main() {
    Application::new()
        .theme(theme())
        .window(|ctx| {
            Window::new()
                //.borderless(true)
                .h_align("center")
                .title("Calendar")
                .position((5.0, 5.0))
                .size(WINDOW_WIDTH, WINDOW_HEIGHT)
                .resizeable(true)
                .child(CalendarView::new().build(ctx))
                .build(ctx)
        })
        .run();
}
