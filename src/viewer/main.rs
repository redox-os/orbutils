#![deny(warnings)]

extern crate orbclient;
extern crate png;

use std::cmp::max;

use std::env;
use std::slice;

use orbclient::*;

fn event_loop(window: &mut Box<Window>){
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

fn error_msg(window: &mut Box<Window>, msg: &str) {
    let mut x = 0;
    for c in msg.chars() {
        window.char(x, 0, c, Color::rgb(255, 255, 255));
        x += 8;
    }
}

fn main() {
    let url = match env::args().nth(1) {
        Some(arg) => arg,
        None => "none:".to_string(),
    };

    if url.ends_with(".bmp") {
        let bmp = BmpFile::from_path(&url);
        let mut window = Window::new(-1,
                                     -1,
                                     max(320, bmp.width() as u32),
                                     max(32, bmp.height() as u32),
                                     &("Viewer (".to_string() + &url + ")"))
                             .unwrap();
        window.set(Color::rgb(0, 0, 0));
        window.image(0, 0, bmp.width() as u32, bmp.height() as u32, &bmp);
        window.sync();
        event_loop(&mut window);
    } else if url.ends_with(".png") {
        let png = png::load_png(&url).unwrap();
        let mut window = Window::new(-1,
                                     -1,
                                     max(320, png.width),
                                     max(32, png.height),
                                     &("Viewer (".to_string() + &url + ")"))
                             .unwrap();
        window.set(Color::rgb(0, 0, 0));
        match png.pixels {
            png::PixelsByColorType::K8(_data) => error_msg(&mut window, "Does not support K8"),
            png::PixelsByColorType::KA8(_data) => error_msg(&mut window, "Does not support KA8"),
            png::PixelsByColorType::RGB8(_data) => error_msg(&mut window, "Does not support RGB8"),
            png::PixelsByColorType::RGBA8(data) => {
                let data_colors = unsafe { slice::from_raw_parts(data.as_ptr() as *const Color, data.len()/4) };
                window.image(0, 0, png.width, png.height, data_colors);
            }
        }
        window.sync();
        event_loop(&mut window);
    } else {
        let mut window = Window::new(-1,
                                     -1,
                                     320,
                                     32,
                                     &("Viewer (".to_string() + &url + ")"))
                             .unwrap();
        window.set(Color::rgb(0, 0, 0));
        error_msg(&mut window, "Unknown image type");
        window.sync();
        event_loop(&mut window);
    }
}
