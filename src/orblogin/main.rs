#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;
extern crate orbimage;
extern crate orbfont;
extern crate orbtk;
extern crate userutils;

use std::{env, str};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Command;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbimage::Image;
use userutils::Passwd;

#[derive(Clone, Copy)]
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

fn login_command(user: &str, pass: &str, launcher_cmd: &str, launcher_args: &[String]) -> Option<Command> {
    let mut passwd_string = String::new();
    File::open("/etc/passwd").unwrap().read_to_string(&mut passwd_string).unwrap();

    let mut passwd_option = None;
    for line in passwd_string.lines() {
        if let Ok(passwd) = Passwd::parse(line) {
            if user == passwd.user && "" == passwd.hash {
                passwd_option = Some(passwd);
                break;
            }
        }
    }

    if passwd_option.is_none() {
        for line in passwd_string.lines() {
            if let Ok(passwd) = Passwd::parse(line) {
                if user == passwd.user && passwd.verify(&pass) {
                    passwd_option = Some(passwd);
                    break;
                }
            }
        }
    }

    if let Some(passwd) = passwd_option {
        let mut command = Command::new(&launcher_cmd);
        for arg in launcher_args.iter() {
            command.arg(&arg);
        }

        command.uid(passwd.uid);
        command.gid(passwd.gid);

        command.current_dir(passwd.home);

        command.env("USER", &user);
        command.env("UID", format!("{}", passwd.uid));
        command.env("GROUPS", format!("{}", passwd.gid));
        command.env("HOME", passwd.home);
        command.env("SHELL", passwd.shell);

        Some(command)
    } else {
        None
    }
}

fn login_window(launcher_cmd: &str, launcher_args: &[String]) -> Option<Command> {
    let (display_width, display_height) = orbclient::get_display_size().expect("orblogin: failed to get display size");

    let mut window = Window::new_flags(
        0, 0, display_width, display_height, "",
        &[WindowFlag::Back, WindowFlag::Unclosable]
    ).unwrap();

    let image_mode = BackgroundMode::from_str("zoom");
    let image_path = "/ui/login.png";
    let image = match Image::from_path(image_path) {
        Ok(image) => image,
        Err(err) => {
            println!("orblogin: error loading {}: {}", image_path, err);
            Image::from_color(display_width, display_height, Color::rgb(0, 0, 0))
        }
    };

    let mut item = 0;
    let mut username = String::new();
    let mut password = String::new();
    let mut failure = false;

    let mut key_events = VecDeque::<orbclient::KeyEvent>::new();
    let mut scaled_image = image.clone();
    let mut redraw = true;
    let mut resize = Some((display_width, display_height));
    loop {
        if let Some((w, h)) = resize.take() {
            let (width, height) = find_scale(&image, image_mode, w, h);

            if width == scaled_image.width() && height == scaled_image.height() {
                // Do not resize scaled image
            } else if width == image.width() && height == image.height() {
                scaled_image = image.clone();
            } else {
                scaled_image = image.resize(width, height, orbimage::ResizeType::Lanczos3).unwrap();
            }

            window.set(Color::rgb(0, 0, 0));

            let x = (window.width() - scaled_image.width())/2;
            let y = (window.height() - scaled_image.height())/2;
            scaled_image.draw(&mut window, x as i32, y as i32);

            redraw = true;
        }

        while let Some(key_event) = key_events.pop_front() {
            match key_event.scancode {
                orbclient::K_BKSP => {
                    if item == 0 {
                        username.pop();
                    } else if item == 1 {
                        password.pop();
                    }

                    redraw = true;
                },
                orbclient::K_ENTER => {
                    if item == 0 {
                        item = 1;
                    } else if item == 1 {
                        if let Some(command) = login_command(&username, &password, launcher_cmd, launcher_args) {
                            return Some(command);
                        } else {
                            item = 1;
                            password.clear();
                            failure = true
                        }
                    }

                    redraw = true;
                },
                orbclient::K_ESC => {
                    item = 0;
                    username.clear();
                    password.clear();
                    failure = false;

                    redraw = true;
                },
                orbclient::K_TAB => {
                    if item == 0 {
                        item = 1;
                    } else if item == 1 {
                        item = 0;
                    }

                    redraw = true;
                },
                _ => match key_event.character {
                    '\0' => (),
                    _ => {
                        if item == 0 {
                            username.push(key_event.character);
                        } else if item == 1 {
                            password.push(key_event.character);
                        }

                        redraw = true;
                    }
                }
            }
        }

        if redraw {
            redraw = false;

            let mut x = (display_width as i32 - 200)/2;
            let mut y = (display_height as i32 - 64)/2;

            window.rect(x - 2, y - 2, 204, 28, if item == 0 {
                if failure {
                    Color::rgb(255, 0, 0)
                } else {
                    Color::rgb(255, 255, 255)
                }
            } else {
                Color::rgb(128, 128, 128)
            });
            window.rect(x, y, 200, 24, Color::rgb(128, 128, 128));
            x += 4;
            y += 4;

            for c in username.chars() {
                window.char(x, y, c, Color::rgb(255, 255, 255));
                x += 8;
            }

            x = (display_width as i32 - 200)/2;
            y += 32;

            window.rect(x - 2, y - 2, 204, 28, if item == 1 {
                if failure {
                    Color::rgb(255, 0, 0)
                } else {
                    Color::rgb(255, 255, 255)
                }
            } else {
                Color::rgb(128, 128, 128)
            });
            window.rect(x, y, 200, 24, Color::rgb(128, 128, 128));
            x += 4;
            y += 4;

            for _c in password.chars() {
                window.char(x, y, '*', Color::rgb(255, 255, 255));
                x += 8;
            }

            window.sync();
        }

        for event in window.events() {
            match event.to_option() {
                EventOption::Key(key_event) => if key_event.pressed {
                    key_events.push_back(key_event);
                },
                EventOption::Resize(resize_event) => {
                    resize = Some((resize_event.width, resize_event.height));
                },
                EventOption::Quit(_) => return None,
                _ => ()
            }
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let launcher_cmd = args.next().expect("orblogin: no window manager command");
    let launcher_args: Vec<String> = args.collect();

    loop {
        if let Some(mut command) = login_window(&launcher_cmd, &launcher_args) {
            match command.spawn() {
                Ok(mut child) => match child.wait() {
                    Ok(_) => (),
                    Err(err) => {
                        println!("orblogin: failed to wait for '{}': {}", launcher_cmd, err);
                    }
                },
                Err(err) => {
                    println!("orblogin: failed to execute '{}': {}", launcher_cmd, err);
                }
            }
        }
    }
}
