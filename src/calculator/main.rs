extern crate orbtk;
use orbtk::*;

extern crate calc;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

static DARK_EXT: &'static str = include_str!("dark-ext.css");

#[cfg(feature = "light-theme")]
static LIGHT_EXT: &'static str = include_str!("light-ext.css");

#[cfg(not(feature = "light-theme"))]
fn get_theme() -> Theme {
    Theme::create().extension_css(DARK_EXT).build()
}

#[cfg(feature = "light-theme")]
fn get_theme() -> Theme {
    Theme::create_light_theme()
        .extension_css(DARK_EXT)
        .extension_css(LIGHT_EXT)
        .build()
}

#[derive(Default)]
struct MainViewState {
    result: RefCell<String>,
    input: RefCell<String>,
    eval: Cell<bool>,
    clear: Cell<bool>,
    updated: Cell<bool>,
}

impl MainViewState {
    fn clear(&self) {
        self.result.borrow_mut().clear();
        self.input.borrow_mut().clear();
        self.clear.set(true);
        self.updated.set(true);
    }

    fn eval(&self) {
        self.eval.set(true);
        self.updated.set(true);
    }
    fn input(&self, sight: &str) {
        *self.input.borrow_mut() = sight.to_string();
        self.updated.set(true);
    }
}

impl State for MainViewState {
    fn update(&self, context: &mut Context) {
        let mut result = None;

        if let Some(child) = &mut context.child_by_id("input") {
            if let Ok(text) = child.borrow_mut_property::<Text>() {
                if self.clear.get() {
                    text.0.clear();
                    self.clear.set(false);
                } else if self.updated.get() {
                    text.0.push_str(&*self.input.borrow());
                }

                if self.eval.get() {
                    let res = match calc::eval(&text.0) {
                        Ok(s) => s.to_string(),
                        Err(e) => e.into(),
                    };

                    result = Some(res);
                    self.eval.set(false);
                }

                self.input.borrow_mut().clear();
            }
        }

        if let Some(result) = result {
            *self.result.borrow_mut() = result;
        }

        if self.updated.get() || self.clear.get() {
            if let Ok(text) = context.widget().borrow_mut_property::<Text>() {
                text.0 = self.result.borrow().clone();
            }
        }

        self.updated.set(false);
    }
}

fn get_button_selector(primary: bool) -> Selector {
    let selector = Selector::from("button");

    if primary {
        selector.class("primary")
    } else {
        selector
    }
}

fn generate_button(
    state: &Rc<MainViewState>,
    sight: &str,
    primary: bool,
    column: usize,
    row: usize,
) -> ButtonTemplate {
    let sight = String::from(sight);
    let state = state.clone();

    Button::create()
        .size(48.0, 48.0)
        .min_width(0.0)
        .text(sight.clone())
        .selector(get_button_selector(primary))
        .on_click(move |_| -> bool {
            state.input(&String::from(sight.clone()));
            true
        })
        .attach_property(GridColumn(column))
        .attach_property(GridRow(row))
}

fn generate_operation_button(
    sight: &str,
    primary: bool,
    column: usize,
    row: usize,
) -> ButtonTemplate {
    Button::create()
        .size(48.0, 48.0)
        .min_width(0.0)
        .text(sight.to_string())
        .selector(get_button_selector(primary).class("square"))
        .attach_property(GridColumn(column))
        .attach_property(GridRow(row))
}

struct MainView;

impl Widget for MainView {
    type Template = Template;

    fn create() -> Self::Template {
        let state = Rc::new(MainViewState::default());
        let clear_state = state.clone();
        let text = SharedProperty::new(Text::from(""));

        Template::default()
            .debug_name("MainView")
            .state(state.clone())
            .child(
                Grid::create()
                    .rows(Rows::create().row(72.0).row("*").build())
                    .child(
                        Container::create()
                            .padding(8.0)
                            .constraint(Constraint::create().build())
                            .selector(Selector::from("container").class("header"))
                            .attach_property(GridRow(0))
                            .child(
                                Grid::create()
                                    .child(
                                        TextBox::create()
                                            .height(14.0)
                                            .padding(0.0)
                                            .selector(Selector::from("textbox").id("input"))
                                            .vertical_alignment("Start"),
                                    )
                                    .child(
                                        TextBlock::create()
                                            .shared_text(text.clone())
                                            .vertical_alignment("End")
                                            .horizontal_alignment("End"),
                                    ),
                            ),
                    )
                    .child(
                        Container::create()
                            .selector(Selector::from("container").class("content"))
                            .padding(8.0)
                            .attach_property(GridRow(1))
                            .child(
                                Grid::create()
                                    .columns(
                                        Columns::create()
                                            .column(48.0)
                                            .column(4.0)
                                            .column(48.0)
                                            .column(4.0)
                                            .column(48.0)
                                            .column(4.0)
                                            .column(48.0)
                                            .build(),
                                    )
                                    .rows(
                                        Rows::create()
                                            .row(48.0)
                                            .row(4.0)
                                            .row(48.0)
                                            .row(4.0)
                                            .row(48.0)
                                            .row(4.0)
                                            .row(48.0)
                                            .row(4.0)
                                            .row(48.0)
                                            .build(),
                                    )
                                    // row 0
                                    .child(generate_button(&state, "(", false, 0, 0))
                                    .child(generate_button(&state, ")", false, 2, 0))
                                    .child(generate_button(&state, "^", false, 4, 0))
                                    .child(generate_button(&state, "/", true, 6, 0))
                                    // row 2
                                    .child(generate_button(&state, "7", false, 0, 2))
                                    .child(generate_button(&state, "8", false, 2, 2))
                                    .child(generate_button(&state, "9", false, 4, 2))
                                    .child(generate_button(&state, "*", true, 6, 2))
                                    // row 4
                                    .child(generate_button(&state, "4", false, 0, 4))
                                    .child(generate_button(&state, "5", false, 2, 4))
                                    .child(generate_button(&state, "6", false, 4, 4))
                                    .child(generate_button(&state, "-", true, 6, 4))
                                    // row 6
                                    .child(generate_button(&state, "1", false, 0, 6))
                                    .child(generate_button(&state, "2", false, 2, 6))
                                    .child(generate_button(&state, "3", false, 4, 6))
                                    .child(generate_button(&state, "+", true, 6, 6))
                                    // row 8
                                    .child(generate_button(&state, "0", false, 0, 8))
                                    .child(generate_button(&state, ".", false, 2, 8))
                                    .child(generate_operation_button("C", false, 4, 8).on_click(
                                        move |_| {
                                            clear_state.clear();
                                            true
                                        },
                                    ))
                                    .child(generate_operation_button("=", true, 6, 8).on_click(
                                        move |_| {
                                            state.eval();
                                            true
                                        },
                                    )),
                            ),
                    ),
            )
            .shared_property(text)
    }
}

fn main() {
    let mut application = Application::new();

    application
        .create_window()
        .bounds((0.0, 0.0, 220.0, 344.0))
        .title("Calculator")
        .theme(get_theme())
        .root(MainView::create())
        .debug_flag(false)
        .build();
    application.run();
}
