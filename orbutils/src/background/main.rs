extern crate event;
extern crate orbclient;
extern crate orbimage;
extern crate orbutils;
extern crate syscall;

use event::EventQueue;
use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbimage::Image;
use std::{
    env,
    io,
    os::unix::io::AsRawFd,
    rc::Rc,
};
use syscall::flag::EventFlags;

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
            "center" => BackgroundMode::Center,
            "fill" => BackgroundMode::Fill,
            "scale" => BackgroundMode::Scale,
            _ => BackgroundMode::Zoom,
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

fn find_background() -> String {
    match env::home_dir() {
        Some(home) => {
            for name in &[
                "background.png",
                "background.jpg",
            ] {
                let path = home.join(name);
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        return path_str.to_string();
                    }
                }
            }
        }
        _ => (),
    }

    "/ui/background.png".to_string()
}

fn main() {
    let mut args = env::args().skip(1);

    let path = match args.next() {
        Some(arg) => arg,
        None => find_background(),
    };

    let mode = BackgroundMode::from_str(&args.next().unwrap_or_default());

    match Image::from_path(&path).map(Rc::new) {
        Ok(image) => {
            let mut event_queue = EventQueue::<()>::new().expect("background: failed to create event queue");

            for display in orbutils::get_display_rects().expect("background: failed to get display rects") {
                let mut window = Window::new_flags(
                    display.x, display.y, display.width, display.height, "",
                    &[WindowFlag::Async, WindowFlag::Back, WindowFlag::Borderless, WindowFlag::Unclosable]
                ).unwrap();

                let image = image.clone();
                let mut scaled_image = (*image).clone();
                let mut resize = Some((display.width, display.height));
                event_queue.add(window.as_raw_fd(), move |_event| -> io::Result<Option<()>> {
                    for event in window.events() {
                        match event.to_option() {
                            EventOption::Resize(resize_event) => {
                                resize = Some((resize_event.width, resize_event.height));
                            },
                            EventOption::Screen(screen_event) => {
                                window.set_size(screen_event.width, screen_event.height);
                                resize = Some((screen_event.width, screen_event.height));
                            },
                            _ => ()
                        }
                    }

                    if let Some((w, h)) = resize.take() {
                        let (width, height) = find_scale(&image, mode, w, h);

                        if width == scaled_image.width() && height == scaled_image.height() {
                            // Do not resize scaled image
                        } else if width == image.width() && height == image.height() {
                            scaled_image = (*image).clone();
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

                    Ok(None)
                }).expect("background: failed to poll window events");
            }

            event_queue.trigger_all(event::Event {
                fd: 0,
                flags: EventFlags::empty(),
            }).expect("background: failed to trigger events");

            event_queue.run().expect("background: failed to run event loop");
        },
        Err(err) => {
            println!("background: error loading {}: {}", path, err);
        }
    }
}
