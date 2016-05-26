#![deny(warnings)]

extern crate orbclient;
extern crate orbimage;

use std::cmp::max;

use std::env;

use orbclient::{Color, Window, EventOption, K_ESC};
use orbimage::Image;

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
        window.char(x, 0, c, Color::rgb(0, 0, 0));
        x += 8;
    }
}

fn main() {
    let url = match env::args().nth(1) {
        Some(arg) => arg,
        None => "none:".to_string(),
    };

    match Image::from_path(&url) {
        Ok(image) => {
            let mut window = Window::new(-1, -1, max(320, image.width()), max(32, image.height()),
                                         &("Viewer (".to_string() + &url + ")")).unwrap();

            let box_size = 4;
            for box_y in 0..window.height()/box_size {
                for box_x in 0..window.width()/box_size {
                    let color = if box_x % 2 == box_y % 2 {
                        Color::rgb(102, 102, 102)
                    }else{
                        Color::rgb(53, 53, 53)
                    };

                    window.rect((box_x * box_size) as i32, (box_y * box_size) as i32, box_size, box_size, color);
                }
            }

            image.draw(&mut window, 0, 0);
            window.sync();
            event_loop(&mut window);
        },
        Err(err) => {
            let mut window = Window::new(-1,
                                         -1,
                                         320,
                                         32,
                                         &("Viewer (".to_string() + &url + ")"))
                                 .unwrap();
            window.set(Color::rgb(255, 255, 255));
            error_msg(&mut window, &format!("{}", err));
            window.sync();
            event_loop(&mut window);
        }
    }
}
