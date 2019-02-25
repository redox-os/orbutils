#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;
extern crate orbimage;
extern crate orbfont;
extern crate redox_users;

use std::{env, str};
use std::process::Command;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbfont::Font;
use orbimage::Image;
use redox_users::{AllUsers};

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

fn login_command(username: &str, pass: &str, launcher_cmd: &str, launcher_args: &[String]) -> Option<Command> {

    let sys_users = match AllUsers::new(true) {
        Ok(users) => users,
        // Not maybe the best thing to do...
        Err(_) => {
            return None;
        }
    };

    match sys_users.get_by_name(&username) {
        Some(user) => if user.verify_passwd(&pass) {
            let mut command = user.login_cmd(&launcher_cmd);
            for arg in launcher_args.iter() {
                command.arg(&arg);
            }

            Some(command)
        } else {
            None
        },
        None => None
    }
}

fn login_window(launcher_cmd: &str, launcher_args: &[String], font: &Font, image: &Image, image_mode: BackgroundMode) -> Option<Command> {
    let (display_width, display_height) = orbclient::get_display_size().expect("orblogin: failed to get display size");

    let mut window = Window::new_flags(
        0, 0, display_width, display_height, "orblogin",
        &[WindowFlag::Borderless, WindowFlag::Unclosable]
    ).unwrap();

    let mut item = 0;
    let mut username = String::new();
    let mut password = String::new();
    let mut failure = false;

    let mut scaled_image = image.clone();
    let mut mouse_x = 0;
    let mut mouse_y = 0;
    let mut mouse_left = false;
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

            let x = (window.width() as i32 - 216)/2;
            let y = (window.height() as i32 - 164)/2;
            window.rect(x, y, 216, 164, Color::rgba(0, 0, 0, 128));

            font.render("Username:", 16.0).draw(&mut window, x + 8, y + 8, Color::rgb(255, 255, 255));
            font.render("Password:", 16.0).draw(&mut window, x + 8, y + 68, Color::rgb(255, 255, 255));

            redraw = true;
        }

        if redraw {
            redraw = false;

            let active = if failure {
                Color::rgb(255, 0, 0)
            } else {
                Color::rgb(255, 255, 255)
            };
            let inactive = if failure {
                Color::rgb(128, 0, 0)
            } else {
                Color::rgb(29, 29, 29)
            };

            let x = (window.width() as i32 - 200)/2;
            let mut y = (window.height() as i32 - 148)/2;

            y += 24;

            {
                window.rect(x, y, 200, 28, if item == 0 { active } else { inactive });
                window.rect(x + 2, y + 2, 196, 24, Color::rgb(40, 40, 40));
                let mut string = username.to_string();
                if item == 0 {
                    string.push('|');
                }
                font.render(&string, 16.0).draw(&mut window, x + 6, y + 6, Color::rgb(255, 255, 255));
            }

            y += 60;

            {
                window.rect(x, y, 200, 28, if item == 1 { active } else { inactive });
                window.rect(x + 2, y + 2, 196, 24, Color::rgb(40, 40, 40));
                let mut string = String::new();
                for _c in password.chars() {
                    string.push('•');
                }
                if item == 1 {
                    string.push('|');
                }
                font.render(&string, 16.0).draw(&mut window, x + 6, y + 6, Color::rgb(255, 255, 255));
            }

            y += 36;

            {
                window.rect(x, y, 200, 28, Color::rgb(29, 29, 29));
                window.rect(x + 2, y + 2, 196, 24, Color::rgb(39, 72, 105));
                let text = font.render(&"Login", 16.0);
                text.draw(&mut window, x + (200 - text.width() as i32)/2, y + 6, Color::rgb(255, 255, 255));
            }

            window.sync();
        }

        for event in window.events() {
            match event.to_option() {
                EventOption::Key(key_event) => if key_event.pressed {
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
                                    item = 0;
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
                },
                EventOption::Mouse(mouse_event) => {
                    mouse_x = mouse_event.x;
                    mouse_y = mouse_event.y;
                },
                EventOption::Button(button_event) => {
                    if ! button_event.left && mouse_left {
                        let x = (window.width() as i32 - 216)/2;
                        let y = (window.height() as i32 - 164)/2;

                        if mouse_x >= x && mouse_x < x + 216 && mouse_y >= y && mouse_y < y + 164 {
                            if mouse_y < y + 64 {
                                item = 0;
                            } else if mouse_y < y + 128 {
                                item = 1;
                            } else {
                                if let Some(command) = login_command(&username, &password, launcher_cmd, launcher_args) {
                                    return Some(command);
                                } else {
                                    item = 0;
                                    password.clear();
                                    failure = true
                                }
                            }

                            redraw = true;
                        }
                    }
                    mouse_left = button_event.left;
                },
                EventOption::Resize(resize_event) => {
                    resize = Some((resize_event.width, resize_event.height));
                },
                EventOption::Screen(screen_event) => {
                    window.set_size(screen_event.width, screen_event.height);
                    resize = Some((screen_event.width, screen_event.height));
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

    let font = Font::find(Some("Sans"), None, None).expect("orblogin: no font found");

    let image_mode = BackgroundMode::from_str("zoom");
    let image_path = "/ui/login.png";
    let image = match Image::from_path(image_path) {
        Ok(image) => image,
        Err(err) => {
            println!("orblogin: error loading {}: {}", image_path, err);
            Image::from_color(1, 1, Color::rgb(0x2d, 0x64, 0x8e))
        }
    };

    loop {
        if let Some(mut command) = login_window(&launcher_cmd, &launcher_args, &font, &image, image_mode) {
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
