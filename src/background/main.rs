#![deny(warnings)]

extern crate orbclient;
extern crate orbimage;

use std::env;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbimage::Image;

#[derive(Clone, Copy, Debug)]
enum BackgroundMode {
    /// Do not resize the image, just center it
    Center,
    /// Resize the image to the display size
    Fill,
    /// Resize the image - keeping its aspect ratio, and fit it to the display with blank space
    Scale,
    /// Resize the image - keeping its aspect ratio, and crop to remove all blank space
    Zoom,
}

impl BackgroundMode {
    fn from_str(string: &str) -> BackgroundMode {
        match string {
            "fill" => BackgroundMode::Fill,
            "scale" => BackgroundMode::Scale,
            "zoom" => BackgroundMode::Zoom,
            _ => BackgroundMode::Center
        }
    }
}

fn find_scale(image: &Image, mode: BackgroundMode, display_width: u32, display_height: u32) -> (u32, u32) {
    match mode {
        BackgroundMode::Center => {
            (image.width(), image.height())
        },
        BackgroundMode::Fill => {
            (display_width, display_height)
        },
        BackgroundMode::Scale => {
            let d_w = display_width as f64;
            let d_h = display_height as f64;
            let i_w = image.width() as f64;
            let i_h = image.height() as f64;

            let scale = if d_w / d_h > i_w / i_h {
                d_h / i_h
            } else {
                d_w / i_w
            };

            ((i_w * scale) as u32, (i_h * scale) as u32)
        },
        BackgroundMode::Zoom => {
            let d_w = display_width as f64;
            let d_h = display_height as f64;
            let i_w = image.width() as f64;
            let i_h = image.height() as f64;

            let scale = if d_w / d_h < i_w / i_h {
                d_h / i_h
            } else {
                d_w / i_w
            };

            ((i_w * scale) as u32, (i_h * scale) as u32)
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let path = match args.next() {
        Some(arg) => arg,
        None => "/ui/background.png".to_string(),
    };

    let mode = BackgroundMode::from_str(&args.next().unwrap_or(String::new()));

    match Image::from_path(&path) {
        Ok(image) => {
            let (display_width, display_height) = orbclient::get_display_size().expect("background: failed to get display size");

            let mut window = Window::new_flags(
                0, 0, display_width, display_height, &format!("{} - Background", path),
                &[WindowFlag::Back, WindowFlag::Borderless, WindowFlag::Unclosable]
            ).unwrap();

            let mut scaled_image = image.clone();
            let mut resize = Some((display_width, display_height));
            loop {
                if let Some((w, h)) = resize.take() {
                    let (width, height) = find_scale(&image, mode, w, h);

                    if width == scaled_image.width() && height == scaled_image.height() {
                        // Do not resize scaled image
                    } else if width == image.width() && height == image.height() {
                        scaled_image = image.clone();
                    } else {
                        scaled_image = image.resize(width, height, orbimage::ResizeType::Lanczos3).unwrap();
                    }

                    let (crop_x, crop_w) = if width > w  {
                        ((width - w)/2, w)
                    } else {
                        (0, width)
                    };

                    let (crop_y, crop_h) = if height > h {
                        ((height - h)/2, h)
                    } else {
                        (0, height)
                    };

                    window.set(Color::rgb(0, 0, 0));

                    let x = (w as i32 - crop_w as i32)/2;
                    let y = (h as i32 - crop_h as i32)/2;
                    scaled_image.roi(
                        crop_x, crop_y,
                        crop_w, crop_h,
                    ).draw(
                        &mut window,
                        x, y
                    );

                    window.sync();
                }

                for event in window.events() {
                    match event.to_option() {
                        EventOption::Resize(resize_event) => {
                            resize = Some((resize_event.width, resize_event.height));
                        },
                        EventOption::Screen(screen_event) => {
                            window.set_size(screen_event.width, screen_event.height);
                            resize = Some((screen_event.width, screen_event.height));
                        },
                        EventOption::Quit(_) => return,
                        _ => ()
                    }
                }
            }
        },
        Err(err) => {
            println!("background: error loading {}: {}", path, err);
        }
    }
}
