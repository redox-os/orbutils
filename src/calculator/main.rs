extern crate orbtk;
use orbtk::*;

extern crate calc;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

static DARK_THEME_EXTENSION: &'static str = include_str!("dark-theme-extension.css");
// static LIGHT_THEME_EXTENSION: &'static str = include_str!("light-theme-extension.css");

#[derive(Default)]
struct MainViewState {
    result: RefCell<String>,
    updated: Cell<bool>,
}

impl MainViewState {
    fn clear(&self) {
        self.result.borrow_mut().clear();
        self.updated.set(true);
    }

    fn eval(&self) {
        let result = match calc::eval(&*self.result.borrow()) {
            Ok(s) => s.to_string(),
            Err(e) => e.into(),
        };

        (*self.result.borrow_mut()) = result;
        self.updated.set(true);
    }
    fn input(&self, sight: &str) {
        let result = self.result.borrow().clone();
        (*self.result.borrow_mut()) = format!("{}{}", result, sight);
        self.updated.set(true);
    }
}

impl State for MainViewState {
    fn update(&self, context: &mut Context) {
        if let Ok(label) = context.widget().borrow_mut_property::<Label>() {
            if self.updated.get() {
                label.0 = self.result.borrow().clone();
            } else {
                *self.result.borrow_mut() = label.0.clone();
            }

            self.updated.set(false);
        }
    }
}

fn get_button_selector(primary: bool) -> Selector {
    let selector = Selector::from("button").with_class("square");

    if primary {
        selector.with_class("primary")
    } else {
        selector
    }
}

fn generate_button(state: &Rc<MainViewState>, sight: &str, primary: bool) -> Template {
    let sight = String::from(sight);
    let state = state.clone();

    Container::create().with_child(
        Button::create()
            .with_property(Label(sight.clone()))
            .with_property(get_button_selector(primary))
            .with_event_handler(MouseEventHandler::default().on_click(Rc::new(
                move |_pos: Point| -> bool {
                    state.input(&String::from(sight.clone()));
                    true
                },
            ))),
    )
}

fn generate_operation_button(sight: &str, handler: MouseEventHandler, primary: bool) -> Template {
    Container::create().with_child(
        Button::create()
            .with_property(Label(sight.to_string()))
            .with_property(get_button_selector(primary).with_class("square"))
            .with_event_handler(handler),
    )
}

struct MainView;

impl Widget for MainView {
    fn create() -> Template {
        let state = Rc::new(MainViewState::default());
        let clear_state = state.clone();
        let label = SharedProperty::new(Label::from(""));

        Template::default()
            .as_parent_type(ParentType::Single)
            .with_state(state.clone())
            .with_child(
                Column::create()
                    .with_child(
                        Container::create()
                            .with_property(Selector::from("container").with_class("header"))
                            .with_child(TextBox::create().with_shared_property(label.clone())),
                    )
                    .with_child(
                        Container::create()
                            .with_property(Selector::from("container").with_class("content"))
                            .with_child(
                                Column::create()
                                    .with_child(
                                        Row::create()
                                            .with_child(generate_button(&state, "(", false))
                                            .with_child(generate_button(&state, ")", false))
                                            .with_child(generate_button(&state, "^", false))
                                            .with_child(generate_button(&state, "/", true)),
                                    )
                                    .with_child(
                                        Row::create()
                                            .with_child(generate_button(&state, "7", false))
                                            .with_child(generate_button(&state, "8", false))
                                            .with_child(generate_button(&state, "9", false))
                                            .with_child(generate_button(&state, "*", true)),
                                    )
                                    .with_child(
                                        Row::create()
                                            .with_child(generate_button(&state, "4", false))
                                            .with_child(generate_button(&state, "5", false))
                                            .with_child(generate_button(&state, "6", false))
                                            .with_child(generate_button(&state, "-", true)),
                                    )
                                    .with_child(
                                        Row::create()
                                            .with_child(generate_button(&state, "1", false))
                                            .with_child(generate_button(&state, "2", false))
                                            .with_child(generate_button(&state, "3", false))
                                            .with_child(generate_button(&state, "+", true)),
                                    )
                                    .with_child(
                                        Row::create()
                                            .with_child(generate_button(&state, "0", false))
                                            .with_child(generate_button(&state, ".", false))
                                            .with_child(generate_operation_button(
                                                "C",
                                                MouseEventHandler::default().on_click(Rc::new(
                                                    move |_pos: Point| -> bool {
                                                        clear_state.clear();
                                                        true
                                                    },
                                                )),
                                                false,
                                            ))
                                            .with_child(generate_operation_button(
                                                "=",
                                                MouseEventHandler::default().on_click(Rc::new(
                                                    move |_pos: Point| -> bool {
                                                        state.eval();
                                                        true
                                                    },
                                                )),
                                                true,
                                            )),
                                    ),
                            ),
                    ),
            )
            .with_shared_property(label)
    }
}

fn main() {
    let mut application = Application::new();

    let theme = format!("{}{}", DARK_THEME_EXTENSION, DEFAULT_THEME_CSS);
    //let theme = format!("{}{}", LIGHT_THEME_EXTENSION, LIGHT_THEME_CSS);

    application
        .create_window()
        .with_bounds(Bounds::new(0, 0, 220, 364))
        .with_title("Calculator")
        .with_theme(Theme::parse(&theme))
        .with_root(MainView::create())
        .with_debug_flag(false)
        .build();
    application.run();
}
