#![deny(warnings)]

extern crate orbclient;
extern crate orbimage;

use std::cmp::max;
use std::env;

use orbclient::{Color, Renderer, Window, EventOption, K_ESC};
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
    let path = match env::args().nth(1) {
        Some(arg) => arg,
        None => "/ui/background.png".to_string(),
    };

    match Image::from_path(&path) {
        Ok(image) => {
            let (width, height) = orbclient::get_display_size().expect("viewer: failed to get display size");
            println!("Display: {}, {}", width, height);

            println!("Image: {}, {}", image.width(), image.height());

            let best_width = width*3/4;
            let best_height = height*3/4;

            let d_w = best_width as f64;
            let d_h = best_height as f64;
            let i_w = image.width() as f64;
            let i_h = image.height() as f64;

            let best_scale = if d_w / d_h > i_w / i_h {
                d_h / i_h
            } else {
                d_w / i_w
            };

            let (scaled_image, scale) = if best_scale < 1.0 {
                (image.resize((i_w * best_scale) as u32, (i_h * best_scale) as u32,
                             orbimage::ResizeType::Lanczos3).unwrap(), best_scale)
            } else {
                (image.clone(), 1.0)
            };

            let mut window = Window::new(-1, -1, max(320, scaled_image.width()), max(240, scaled_image.height()),
                                         &format!("{} - {:.1}% - Viewer", path, scale * 100.0)).unwrap();

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

            let x = (window.width() - scaled_image.width())/2;
            let y = (window.height() - scaled_image.height())/2;
            scaled_image.draw(&mut window, x as i32, y as i32);
            window.sync();
            event_loop(&mut window);
        },
        Err(err) => {
            let msg = format!("{}", err);
            let mut window = Window::new(-1, -1, max(320, msg.len() as u32 * 8), 32,
                                         &format!("{} - Viewer", path)).unwrap();
            window.set(Color::rgb(255, 255, 255));
            error_msg(&mut window, &msg);
            window.sync();
            event_loop(&mut window);
        }
    }
}
