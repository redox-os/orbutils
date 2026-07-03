#![deny(warnings)]

use std::cmp::max;
use std::env;

use orbclient::image::Image;
use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};

fn find_scale(image: &Image, width: u32, height: u32) -> (u32, u32, f64) {
    let d_w = width as f64;
    let d_h = height as f64;
    let i_w = image.width() as f64;
    let i_h = image.height() as f64;

    let scale = if d_w / d_h > i_w / i_h {
        d_h / i_h
    } else {
        d_w / i_w
    };

    if scale < 1.0 {
        ((i_w * scale) as u32, (i_h * scale) as u32, scale)
    } else {
        (i_w as u32, i_h as u32, 1.0)
    }
}

fn draw_image(window: &mut Window, image: &Image) {
    window.set(Color::rgb(0, 0, 0));
    /*
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
    */

    let x = (window.width() as i32 - image.width() as i32) / 2;
    let y = (window.height() as i32 - image.height() as i32) / 2;
    window.image(x, y, image.width(), image.height(), image.data());
    window.sync();
}

fn main() {
    let path = match env::args().nth(1) {
        Some(arg) => arg,
        None => "/usr/share/ui/background.jpg".to_string(),
    };

    match Image::from_path(&path) {
        Ok(image) => {
            let (display_width, display_height) =
                orbclient::get_display_size().expect("viewer: failed to get display size");

            let (width, height, scale) =
                find_scale(&image, display_width * 4 / 5, display_height * 4 / 5);

            let mut window = Window::new_flags(
                -1,
                -1,
                max(320, width),
                max(240, height),
                &format!("{} - {:.1}% - Viewer", path, scale * 100.0),
                &[WindowFlag::Resizable],
            )
            .unwrap();

            let mut scaled_image = image.clone();
            let mut resize = Some((window.width(), window.height()));
            loop {
                if let Some((w, h)) = resize.take() {
                    let (width, height, scale) = find_scale(&image, w, h);

                    if width == scaled_image.width() && height == scaled_image.height() {
                        // Do not resize scaled image
                    } else if width == image.width() && height == image.height() {
                        scaled_image = image.clone();
                    } else {
                        scaled_image =
                            image.resize(width, height, orbclient::image::ResizeType::Lanczos3);
                    }

                    window.set_title(&format!("{} - {:.1}% - Viewer", path, scale * 100.0));

                    draw_image(&mut window, &scaled_image);
                }

                for event in window.events() {
                    match event.to_option() {
                        EventOption::Resize(resize_event) => {
                            resize = Some((resize_event.width, resize_event.height));
                        }
                        EventOption::Quit(_) => return,
                        _ => (),
                    }
                }
            }
        }
        Err(err) => {
            let msg = err.to_string();

            let mut window = Window::new(
                -1,
                -1,
                max(320, msg.len() as u32 * 8),
                32,
                &format!("{} - Viewer", path),
            )
            .unwrap();

            window.set(Color::rgb(255, 255, 255));

            let mut x = 0;
            for c in msg.chars() {
                window.char(x, 0, c, Color::rgb(0, 0, 0));
                x += 8;
            }

            window.sync();

            loop {
                for event in window.events() {
                    if let EventOption::Quit(_) = event.to_option() {
                        return;
                    }
                }
            }
        }
    }
}
