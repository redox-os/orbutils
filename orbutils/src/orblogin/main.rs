#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

use log::{error, info};
use std::process::Command;
use std::{env, io, str};

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbfont::Font;
use orbimage::Image;
use redox_log::{OutputBuilder, RedoxLogger};
use redox_users::{All, AllUsers, Config};

mod keymap;

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
            _ => BackgroundMode::Center,
        }
    }
}

fn find_scale(
    image: &Image,
    mode: BackgroundMode,
    display_width: u32,
    display_height: u32,
) -> (u32, u32) {
    match mode {
        BackgroundMode::Center => (image.width(), image.height()),
        BackgroundMode::Fill => (display_width, display_height),
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
        }
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

fn normal_usernames() -> Vec<String> {
    let users = match AllUsers::authenticator(Config::default()) {
        Ok(ok) => ok,
        Err(_) => return Vec::new(),
    };

    let mut usernames = Vec::new();
    for user in users.iter() {
        if user.uid >= 1000 {
            usernames.push(user.user.clone());
        }
    }
    usernames.sort();
    usernames
}

fn login_command(
    username: &str,
    pass: &str,
    launcher_cmd: &str,
    launcher_args: &[String],
) -> Option<Command> {
    let sys_users = match AllUsers::authenticator(Config::default()) {
        Ok(users) => users,
        // Not maybe the best thing to do...
        Err(_) => return None,
    };

    match sys_users.get_by_name(&username) {
        Some(user) => {
            if user.verify_passwd(&pass) {
                let mut command = user.login_cmd(&launcher_cmd);
                for arg in launcher_args.iter() {
                    command.arg(&arg);
                }

                Some(command)
            } else {
                None
            }
        }
        None => None,
    }
}

fn login_window(launcher_cmd: &str, launcher_args: &[String]) -> Result<Option<Command>, String> {
    let font = Font::find(Some("Sans"), None, None)?;

    let image_mode = BackgroundMode::from_str("zoom");
    let image_path = "/ui/login.png";
    let image = Image::from_path(image_path)?;

    let (display_width, display_height) = orbclient::get_display_size()?;
    let s_u = (display_height / 1600) + 1;
    let s_i = s_u as i32;
    let s_f = s_i as f32;

    let btn_size_u = 28 * s_u;
    let btn_size_i = 28 * s_i;
    let padding = 10 * s_i;
    let btn_inner_x = 2 * s_i;
    let btn_inner_y = 2 * s_i;
    let btn_inner_w = (btn_size_i - 4 * s_i) as u32;
    let btn_inner_h = (btn_size_i - 4 * s_i) as u32;
    let btn_text_offset_y = 6 * s_i;
    let btn_color_inactive = Color::rgb(39, 72, 105);
    let btn_color_active = Color::rgb(59, 102, 135);
    let btn_border_color = Color::rgb(29, 29, 29);
    let item_height_u = 28 * s_u;
    let item_height_i = 28 * s_i;
    let menu_width_u = 150 * s_u;
    let menu_width_i = 150 * s_i;

    let usernames = normal_usernames();

    let mut window = Window::new_flags(
        0,
        0,
        display_width,
        display_height,
        "orblogin",
        &[WindowFlag::Borderless, WindowFlag::Unclosable],
    )
    .ok_or("Could not create new window with flags")?;

    let (mut item, mut username) = if usernames.len() == 1 {
        (1, usernames[0].clone())
    } else {
        (0, String::new())
    };
    let mut password = String::new();
    let mut failure = false;

    let mut keymap_dropdown_open = false;
    let mut power_dropdown_open = false;
    let mut keymap_state = crate::keymap::KeymapState::new();
    let keymap_options = keymap_state.list.clone();
    let power_options = vec!["Restart", "Shutdown"];

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
                scaled_image = image.resize(width, height, orbimage::ResizeType::Lanczos3)?;
            }

            let (crop_x, crop_w) = if width > w {
                ((width - w) / 2, w)
            } else {
                (0, width)
            };

            let (crop_y, crop_h) = if height > h {
                ((height - h) / 2, h)
            } else {
                (0, height)
            };

            window.set(Color::rgb(0, 0, 0));

            let x = (w as i32 - crop_w as i32) / 2;
            let y = (h as i32 - crop_h as i32) / 2;
            scaled_image
                .roi(crop_x, crop_y, crop_w, crop_h)
                .draw(&mut window, x, y);

            let x = (window.width() as i32 - 216 * s_i) / 2;
            let y = (window.height() as i32 - 164 * s_i) / 2;
            window.rect(x, y, 216 * s_u, 164 * s_u, Color::rgba(0, 0, 0, 128));

            font.render("Username:", 16.0 * s_f).draw(
                &mut window,
                x + 8 * s_i,
                y + 8 * s_i,
                Color::rgb(255, 255, 255),
            );
            font.render("Password:", 16.0 * s_f).draw(
                &mut window,
                x + 8 * s_i,
                y + 68 * s_i,
                Color::rgb(255, 255, 255),
            );

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

            let x = (window.width() as i32 - 200 * s_i) / 2;
            let mut y = (window.height() as i32 - 148 * s_i) / 2;

            y += 24 * s_i;

            // --- Username Box ---
            {
                window.rect(
                    x,
                    y,
                    200 * s_u,
                    28 * s_u,
                    if item == 0 { active } else { inactive },
                );
                window.rect(
                    x + 2 * s_i,
                    y + 2 * s_i,
                    196 * s_u,
                    24 * s_u,
                    Color::rgb(40, 40, 40),
                );
                let mut string = username.to_string();
                if item == 0 {
                    string.push('|');
                }
                font.render(&string, 16.0 * s_f).draw(
                    &mut window,
                    x + 6 * s_i,
                    y + 6 * s_i,
                    Color::rgb(255, 255, 255),
                );
            }

            y += 60 * s_i;

            // --- Password Box ---
            {
                window.rect(
                    x,
                    y,
                    200 * s_u,
                    28 * s_u,
                    if item == 1 { active } else { inactive },
                );
                window.rect(
                    x + 2 * s_i,
                    y + 2 * s_i,
                    196 * s_u,
                    24 * s_u,
                    Color::rgb(40, 40, 40),
                );
                let mut string = String::new();
                for _c in password.chars() {
                    string.push('•');
                }
                if item == 1 {
                    string.push('|');
                }
                font.render(&string, 16.0 * s_f).draw(
                    &mut window,
                    x + 6 * s_i,
                    y + 6 * s_i,
                    Color::rgb(255, 255, 255),
                );
            }

            y += 36 * s_i;

            // --- Login Button ---
            {
                window.rect(x, y, 200 * s_u, 28 * s_u, Color::rgb(29, 29, 29));
                window.rect(
                    x + 2 * s_i,
                    y + 2 * s_i,
                    196 * s_u,
                    24 * s_u,
                    Color::rgb(39, 72, 105),
                );
                let text = font.render(&"Login", 16.0 * s_f);
                text.draw(
                    &mut window,
                    x + (200 * s_i - text.width() as i32) / 2,
                    y + 6 * s_i,
                    Color::rgb(255, 255, 255),
                );
            }

            // --- Buttons and Menus ---
            {
                let power_btn_y = window.height() as i32 - btn_size_i - padding;
                let power_btn_x = window.width() as i32 - btn_size_i - padding;
                let keymap_btn_y = power_btn_y;
                let keymap_btn_x = power_btn_x - btn_size_i - padding;

                let keymap_hover = mouse_x >= keymap_btn_x
                    && mouse_x < keymap_btn_x + btn_size_i
                    && mouse_y >= keymap_btn_y
                    && mouse_y < keymap_btn_y + btn_size_i;
                let power_hover = mouse_x >= power_btn_x
                    && mouse_x < power_btn_x + btn_size_i
                    && mouse_y >= power_btn_y
                    && mouse_y < power_btn_y + btn_size_i;

                let keymap_color = if keymap_hover || keymap_dropdown_open {
                    btn_color_active
                } else {
                    btn_color_inactive
                };
                let power_color = if power_hover || power_dropdown_open {
                    btn_color_active
                } else {
                    btn_color_inactive
                };

                if keymap_state.is_available() {
                    window.rect(
                        keymap_btn_x,
                        keymap_btn_y,
                        btn_size_u,
                        btn_size_u,
                        btn_border_color,
                    );
                    window.rect(
                        keymap_btn_x + btn_inner_x,
                        keymap_btn_y + btn_inner_y,
                        btn_inner_w,
                        btn_inner_h,
                        keymap_color,
                    );
                    let text_k = font.render("K", 16.0 * s_f);
                    text_k.draw(
                        &mut window,
                        keymap_btn_x + (btn_size_i - text_k.width() as i32) / 2,
                        keymap_btn_y + btn_text_offset_y,
                        Color::rgb(255, 255, 255),
                    );
                }
                {
                    window.rect(
                        power_btn_x,
                        power_btn_y,
                        btn_size_u,
                        btn_size_u,
                        btn_border_color,
                    );
                    window.rect(
                        power_btn_x + btn_inner_x,
                        power_btn_y + btn_inner_y,
                        btn_inner_w,
                        btn_inner_h,
                        power_color,
                    );
                    let text_p = font.render("P", 16.0 * s_f);
                    text_p.draw(
                        &mut window,
                        power_btn_x + (btn_size_i - text_p.width() as i32) / 2,
                        power_btn_y + btn_text_offset_y,
                        Color::rgb(255, 255, 255),
                    );
                }
                if keymap_dropdown_open {
                    let menu_height_u = item_height_u * keymap_options.len() as u32;
                    let menu_x = keymap_btn_x - (menu_width_i - btn_size_i); // Align right
                    let menu_y = keymap_btn_y - menu_height_u as i32; // Opens upwards

                    window.rect(
                        menu_x,
                        menu_y,
                        menu_width_u,
                        menu_height_u,
                        btn_border_color,
                    );

                    for (i, option) in keymap_options.iter().enumerate() {
                        let item_y = menu_y + (i as i32 * item_height_i);
                        let item_hover = mouse_x >= menu_x
                            && mouse_x < menu_x + menu_width_i
                            && mouse_y >= item_y
                            && mouse_y < item_y + item_height_i;
                        let item_color = if item_hover || option == &keymap_state.active {
                            btn_color_active
                        } else {
                            btn_color_inactive
                        };
                        window.rect(
                            menu_x + btn_inner_x,
                            item_y + btn_inner_y,
                            (menu_width_i - 4 * s_i) as u32,
                            (item_height_i - 4 * s_i) as u32,
                            item_color,
                        );
                        font.render(option, 16.0 * s_f).draw(
                            &mut window,
                            menu_x + 6 * s_i,
                            item_y + 6 * s_i,
                            Color::rgb(255, 255, 255),
                        );
                    }
                }

                if power_dropdown_open {
                    let menu_height_u = item_height_u * power_options.len() as u32;
                    let menu_x = power_btn_x - (menu_width_i - btn_size_i); // Align right
                    let menu_y = power_btn_y - menu_height_u as i32; // Opens upwards

                    window.rect(
                        menu_x,
                        menu_y,
                        menu_width_u,
                        menu_height_u,
                        btn_border_color,
                    );

                    for (i, option) in power_options.iter().enumerate() {
                        let item_y = menu_y + (i as i32 * item_height_i);
                        let item_hover = mouse_x >= menu_x
                            && mouse_x < menu_x + menu_width_i
                            && mouse_y >= item_y
                            && mouse_y < item_y + item_height_i;
                        let item_color = if item_hover {
                            btn_color_active
                        } else {
                            btn_color_inactive
                        };
                        window.rect(
                            menu_x + btn_inner_x,
                            item_y + btn_inner_y,
                            (menu_width_i - 4 * s_i) as u32,
                            (item_height_i - 4 * s_i) as u32,
                            item_color,
                        );
                        font.render(option, 16.0 * s_f).draw(
                            &mut window,
                            menu_x + 6 * s_i,
                            item_y + 6 * s_i,
                            Color::rgb(255, 255, 255),
                        );
                    }
                }
            }

            window.sync();
        }

        for event in window.events() {
            match event.to_option() {
                EventOption::Key(key_event) => {
                    if key_event.pressed {
                        if keymap_dropdown_open || power_dropdown_open {
                            match key_event.scancode {
                                orbclient::K_ENTER | orbclient::K_ESC | orbclient::K_TAB => {
                                    keymap_dropdown_open = false;
                                    power_dropdown_open = false;
                                    redraw = true;
                                }
                                _ => (),
                            }
                        }

                        match key_event.scancode {
                            orbclient::K_BKSP => {
                                if item == 0 {
                                    username.pop();
                                } else if item == 1 {
                                    password.pop();
                                }

                                redraw = true;
                            }
                            orbclient::K_ENTER => {
                                if item == 0 {
                                    item = 1;
                                } else if item == 1 {
                                    if let Some(command) = login_command(
                                        &username,
                                        &password,
                                        launcher_cmd,
                                        launcher_args,
                                    ) {
                                        return Ok(Some(command));
                                    } else {
                                        item = 0;
                                        password.clear();
                                        failure = true
                                    }
                                }

                                redraw = true;
                            }
                            orbclient::K_ESC => {
                                item = 0;
                                username.clear();
                                password.clear();
                                failure = false;

                                redraw = true;
                            }
                            orbclient::K_TAB => {
                                if item == 0 {
                                    item = 1;
                                } else if item == 1 {
                                    item = 0;
                                }

                                redraw = true;
                            }
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
                            },
                        }
                    }
                }
                EventOption::Mouse(mouse_event) => {
                    mouse_x = mouse_event.x;
                    mouse_y = mouse_event.y;

                    redraw = true;
                }
                EventOption::Button(button_event) => {
                    if !button_event.left && mouse_left {
                        let power_btn_y = window.height() as i32 - btn_size_i - padding;
                        let power_btn_x = window.width() as i32 - btn_size_i - padding;
                        let keymap_btn_y = power_btn_y;
                        let keymap_btn_x = power_btn_x - btn_size_i - padding;

                        let mut trigger_redraw = || {
                            redraw = true;
                            resize = Some((window.width(), window.height()));
                        };

                        if keymap_dropdown_open {
                            let menu_height_i = item_height_i * keymap_options.len() as i32;
                            let menu_x = keymap_btn_x - (menu_width_i - btn_size_i); // Align right
                            let menu_y = keymap_btn_y - menu_height_i;

                            if mouse_x >= menu_x
                                && mouse_x < menu_x + menu_width_i
                                && mouse_y >= menu_y
                                && mouse_y < menu_y + menu_height_i
                            {
                                let item_index = (mouse_y - menu_y) / item_height_i;
                                if let Some(option) = keymap_options.get(item_index as usize) {
                                    keymap_state.set_active(option);
                                }
                                keymap_dropdown_open = false;
                                trigger_redraw();
                                continue;
                            }
                        }

                        if power_dropdown_open {
                            let menu_height_i = item_height_i * power_options.len() as i32;
                            let menu_x = power_btn_x - (menu_width_i - btn_size_i); // Align right
                            let menu_y = power_btn_y - menu_height_i;

                            if mouse_x >= menu_x
                                && mouse_x < menu_x + menu_width_i
                                && mouse_y >= menu_y
                                && mouse_y < menu_y + menu_height_i
                            {
                                let item_index = (mouse_y - menu_y) / item_height_i;
                                if let Some(option) = power_options.get(item_index as usize) {
                                    if *option == "Shutdown" {
                                        Command::new("shutdown").spawn().ok();
                                    } else if *option == "Restart" {
                                        Command::new("shutdown").arg("-r").spawn().ok();
                                    }
                                }
                                power_dropdown_open = false;
                                trigger_redraw();
                                continue;
                            }
                        }

                        if mouse_x >= keymap_btn_x
                            && mouse_x < keymap_btn_x + btn_size_i
                            && mouse_y >= keymap_btn_y
                            && mouse_y < keymap_btn_y + btn_size_i
                        {
                            keymap_dropdown_open = !keymap_dropdown_open;
                            power_dropdown_open = false;
                            trigger_redraw();
                            continue;
                        } else if mouse_x >= power_btn_x
                            && mouse_x < power_btn_x + btn_size_i
                            && mouse_y >= power_btn_y
                            && mouse_y < power_btn_y + btn_size_i
                        {
                            power_dropdown_open = !power_dropdown_open;
                            keymap_dropdown_open = false;
                            trigger_redraw();
                            continue;
                        } else {
                            let x = (window.width() as i32 - 216 * s_i) / 2;
                            let y = (window.height() as i32 - 164 * s_i) / 2;

                            if mouse_x >= x
                                && mouse_x < x + 216 * s_i
                                && mouse_y >= y
                                && mouse_y < y + 164 * s_i
                            {
                                if mouse_y < y + 64 * s_i {
                                    item = 0;
                                } else if mouse_y < y + 128 * s_i {
                                    item = 1;
                                } else {
                                    if let Some(command) = login_command(
                                        &username,
                                        &password,
                                        launcher_cmd,
                                        launcher_args,
                                    ) {
                                        return Ok(Some(command));
                                    } else {
                                        item = 0;
                                        password.clear();
                                        failure = true
                                    }
                                }
                                trigger_redraw();
                                continue;
                            }
                        }

                        if keymap_dropdown_open || power_dropdown_open {
                            keymap_dropdown_open = false;
                            power_dropdown_open = false;
                            trigger_redraw();
                            continue;
                        }
                    }
                    mouse_left = button_event.left;
                }
                EventOption::Resize(resize_event) => {
                    resize = Some((resize_event.width, resize_event.height));
                }
                EventOption::Screen(screen_event) => {
                    window.set_size(screen_event.width, screen_event.height);
                    resize = Some((screen_event.width, screen_event.height));
                }
                EventOption::Quit(_) => return Ok(None),
                _ => (),
            }
        }
    }
}

fn main() -> io::Result<()> {
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Warn)
                .with_ansi_escape_codes()
                .build(),
        )
        .with_process_name("orblogin".into())
        .enable();

    let mut args = env::args().skip(1);

    let launcher_cmd = args.next().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Could not get 'launcher_cmd'",
    ))?;
    let launcher_args: Vec<String> = args.collect();

    loop {
        match login_window(&launcher_cmd, &launcher_args) {
            Ok(Some(mut command)) => match command.spawn() {
                Ok(mut child) => match child.wait() {
                    Ok(_) => (),
                    Err(err) => error!("failed to wait for '{}': {}", launcher_cmd, err),
                },
                Err(err) => error!("failed to execute '{}': {}", launcher_cmd, err),
            },
            Ok(None) => info!("login completed without a command"),
            Err(e) => error!("{}", e),
        }
    }
}
