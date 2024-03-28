extern crate orbclient;
extern crate orbimage;
extern crate redox_log;
extern crate log;
extern crate event;
extern crate libredox;
extern crate dirs;

use std::{
    collections::HashMap, env, fs::File, os::unix::io::{AsRawFd, FromRawFd, RawFd}, rc::Rc
};
use libredox::flag;
use log::error;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbimage::Image;
use redox_log::{OutputBuilder, RedoxLogger};

use event::RawEventQueue;

struct DisplayRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

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
    match dirs::home_dir() {
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

fn get_full_url(path: &str) -> Result<String, String> {
    let file = match libredox::call::open(path, flag::O_CLOEXEC | flag::O_PATH, 0) {
        Ok(ok) => unsafe { File::from_raw_fd(ok as RawFd) },
        Err(err) => return Err(format!("{}", err)),
    };

    let mut buf: [u8; 4096] = [0; 4096];
    let count = libredox::call::fpath(file.as_raw_fd() as usize, &mut buf)
        .map_err(|err| format!("{}", err))?;

    String::from_utf8(Vec::from(&buf[..count]))
        .map_err(|err| format!("{}", err))
}

//TODO: determine x, y of display by talking to orbital instead of guessing!
fn get_display_rects() -> Result<Vec<DisplayRect>, String> {
    let url = get_full_url(
        &env::var("DISPLAY").or(Err("DISPLAY not set"))?
    )?;

    let mut url_parts = url.split(':');
    let scheme_name = url_parts.next().ok_or(format!("no scheme name"))?;
    let path = url_parts.next().ok_or(format!("no path"))?;

    let mut path_parts = path.split('/');
    let vt_screen = path_parts.next().unwrap_or("");
    let width = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);
    let height = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);

    let mut display_rects = vec![DisplayRect {
        x: 0,
        y: 0,
        width,
        height,
    }];

    // If display server supports multiple displays in a VT
    if vt_screen.contains('.') {
        // Look for other screens in the same VT
        let mut parts = vt_screen.split('.');
        let vt_i = parts.next().unwrap_or("").parse::<usize>().unwrap_or(0);
        let start_screen_i = parts.next().unwrap_or("").parse::<usize>().unwrap_or(0);
        //TODO: determine maximum number of screens
        for screen_i in start_screen_i + 1..1024 {
            let url = match get_full_url(&format!("{}:{}.{}", scheme_name, vt_i, screen_i)) {
                Ok(ok) => ok,
                //TODO: only check for ENOENT?
                Err(_err) => break,
            };

            let mut url_parts = url.split(':');
            let _scheme_name = url_parts.next().ok_or(format!("no scheme name"))?;
            let path = url_parts.next().ok_or(format!("no path"))?;

            let mut path_parts = path.split('/');
            let _vt_screen = path_parts.next().unwrap_or("");
            let width = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);
            let height = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);

            let x = if let Some(last) = display_rects.last() {
                last.x + last.width as i32
            } else {
                0
            };

            display_rects.push(DisplayRect {
                x,
                y: 0,
                width,
                height,
            });
        }
    }

    Ok(display_rects)
}

fn main() {
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("background".into())
        .enable();

    let mut args = env::args().skip(1);

    let path = match args.next() {
        Some(arg) => arg,
        None => find_background(),
    };

    let mode = BackgroundMode::from_str(&args.next().unwrap_or_default());

    match Image::from_path(&path).map(Rc::new) {
        Ok(image) => {
            let event_queue = RawEventQueue::new().expect("background: failed to create event queue");

            let mut handlers = HashMap::<usize, Box<dyn FnMut()>>::new();

            for display in get_display_rects().expect("background: failed to get display rects") {
                let mut window = Window::new_flags(
                    display.x, display.y, display.width, display.height, "",
                    &[WindowFlag::Async, WindowFlag::Back, WindowFlag::Borderless, WindowFlag::Unclosable]
                ).unwrap();

                let image = image.clone();
                let mut scaled_image = (*image).clone();
                let mut resize = Some((display.width, display.height));

                event_queue.subscribe(window.as_raw_fd() as usize, window.as_raw_fd() as usize, event::EventFlags::READ)
                    .expect("background: failed to add event");

                let window_raw_fd = window.as_raw_fd();
                let mut handler: Box<dyn FnMut()> = Box::new(move || {
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
                });
                handler();
                handlers.insert(window_raw_fd as usize, handler);
            }

            for event in event_queue.map(|e| e.expect("background: failed to get next event")) {
                let Some(handler) = handlers.get_mut(&event.fd) else {
                    continue;
                };
                (*handler)();
            }
        },
        Err(err) => {
            error!("error loading {}: {}", path, err);
        }
    }
}
