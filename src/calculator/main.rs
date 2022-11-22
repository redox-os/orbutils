#![deny(warnings)]

extern crate orbtk;
extern crate calc;

use orbtk::{Button, Point, Rect, TextBox, Window};
use orbtk::traits::{Click, Enter, Place, Text};

fn eval(input: &str) -> String {
    match calc::eval(input) {
        Ok(s) => s.to_string(),
        Err(e) => e.into()
    }
}

fn main(){
    let mut window = Window::new(Rect::new(-1, -1, 148, 210), "Calculator");

    {
        let text_box = TextBox::new();
        text_box.position(0, 0)
            .size(300, 26)
            .text_offset(4, 4)
            .on_enter(move |text_box: &TextBox| {
                let input = text_box.text.get();
                if ! input.is_empty() {
                    let result = eval(&input);
                    text_box.text_i.set(result.len());
                    text_box.text.set(result);
                }
            });
        window.add(&text_box);

        let mut col = 0;
        let mut row = 0;

        {
            let mut btn = |name| {
                let text_box_clone = text_box.clone();
                let button = Button::new();
                button.position(col * 36 + 4, row * 36 + 30)
                    .size(32, 32)
                    .text(name)
                    .text_offset(12, 8)
                    .on_click(move |_button: &Button, _point: Point| {
                        let text_i = text_box_clone.text_i.get();

                        let text = text_box_clone.text.get();

                        let mut new_text = String::new();
                        new_text.push_str(&text[.. text_i]);
                        new_text.push_str(name);
                        new_text.push_str(&text[text_i ..]);

                        text_box_clone.text.set(new_text);

                        text_box_clone.text_i.set(text_i + name.len());
                    });
                window.add(&button);

                col += 1;
                if col >= 4 {
                    col = 0;
                    row += 1;
                }
            };

            btn("("); btn(")"); btn("**"); btn("/");
            btn("7"); btn("8"); btn("9"); btn("*");
            btn("4"); btn("5"); btn("6"); btn("-");
            btn("1"); btn("2"); btn("3"); btn("+");
            btn("0"); btn(".");
        }

        {
            let text_box_clone = text_box.clone();
            let button = Button::new();
            button.position(col * 36 + 4, row * 36 + 30)
                .size(32, 32)
                .text("C")
                .text_offset(12, 8)
                .on_click(move |_button: &Button, _point: Point| {
                    text_box_clone.text("".to_string());
                });
            window.add(&button);

            col += 1;
        }

        {
            let text_box_clone = text_box.clone();
            let button = Button::new();
            button.position(col * 36 + 4, row * 36 + 30)
                .size(32, 32)
                .text("=")
                .text_offset(12, 8)
                .on_click(move |_button: &Button, _point: Point| {
                    text_box_clone.emit_enter();
                });
            window.add(&button);
        }
    }

    window.exec();
}
