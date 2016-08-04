#![deny(warnings)]

extern crate orbclient;
extern crate orbfont;

use std::cmp::max;

use std::env;

use orbclient::{Color, Window, EventOption, K_ESC};
use orbfont::Font;

fn event_loop(window: &mut Window){
    loop {
        for event in window.events() {
            if let EventOption::Key(key_event) = event.to_option() {
                if key_event.pressed && key_event.scancode == K_ESC {
                    return;
                }
            }
            if let EventOption::Quit(_) = event.to_option() {
                return;
            }
        }
    }
}

fn error_msg(window: &mut Window, msg: &str) {
    let mut x = 0;
    for c in msg.chars() {
        window.char(x, 0, c, Color::rgb(255, 255, 255));
        x += 8;
    }
}

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
            let mut window = Window::new(-1,
                                         -1,
                                         max(320, width),
                                         max(32, height),
                                         &("Character Map (".to_string() + &title + ")"))
                                 .unwrap();
            window.set(Color::rgb(255, 255, 255));
            let mut y = 0;
            for line in lines.iter() {
                line.draw(&mut window, 0, y, Color::rgb(0, 0, 0));
                y += line.height() as i32;
            }
            window.sync();
            event_loop(&mut window);
        },
        Err(err) => {
            let mut window = Window::new(-1,
                                         -1,
                                         320,
                                         32,
                                         &("Character Map (".to_string() + &title + ")"))
                                 .unwrap();
            window.set(Color::rgb(0, 0, 0));
            error_msg(&mut window, &format!("{}", err));
            window.sync();
            event_loop(&mut window);
        }
    }
}
