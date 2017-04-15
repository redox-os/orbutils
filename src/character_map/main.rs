#![deny(warnings)]

extern crate orbclient;
extern crate orbfont;

use std::cmp::max;

use std::env;

use orbclient::{Color, Renderer, Window, WindowFlag, EventOption};
use orbfont::Font;

fn main() {
    let (title, font_res) = match env::args().nth(1) {
        Some(arg) => (arg.clone(), Font::from_path(&arg)),
        None => ("Default Font".to_string(), Font::find(None, None, None)),
    };

    match font_res {
        Ok(font) => {
            let lines = [
                font.render("ABCDEFGHIJK", 64.0),
                font.render("LMNOPQRSTUV", 64.0),
                font.render("WXYZabcdefg", 64.0),
                font.render("hijklmnopqr", 64.0),
                font.render("stuvwxyz.?!", 64.0),
                font.render("0123456789 ", 64.0)
            ];

            let mut width = 0;
            let mut height = 0;
            for line in lines.iter() {
                width = max(width, line.width());
                height += line.height();
            }

            let redraw = move |window: &mut Window| {
                window.set(Color::rgb(255, 255, 255));
                let mut y = 0;
                for line in lines.iter() {
                    line.draw(window, 0, y, Color::rgb(0, 0, 0));
                    y += line.height() as i32;
                }
                window.sync();
            };

            let mut window = Window::new_flags(-1, -1, max(320, width), max(32, height),
                                        &format!("{} - Character Map", title),
                                        &[WindowFlag::Resizable])
                                    .unwrap();

            redraw(&mut window);

            loop {
                for event in window.events() {
                    match event.to_option() {
                        EventOption::Resize(_) => redraw(&mut window),
                        EventOption::Quit(_) => return,
                        _ => ()
                    }
                }
            }
        },
        Err(err) => {
            let mut window = Window::new(-1, -1, 320, 32, &format!("{} - Character Map", title))
                                    .unwrap();
            window.set(Color::rgb(0, 0, 0));
            let mut x = 0;
            for c in format!("{}", err).chars() {
                window.char(x, 0, c, Color::rgb(255, 255, 255));
                x += 8;
            }
            window.sync();
            loop {
                for event in window.events() {
                    match event.to_option() {
                        EventOption::Quit(_) => return,
                        _ => ()
                    }
                }
            }
        }
    }
}
