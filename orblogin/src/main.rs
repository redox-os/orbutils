#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

#[allow(clippy::all)]
mod generated_code {
    slint::include_modules!();
}

pub use generated_code::*;
use redox_users::{All, AllUsers, Config};
use slint::{invoke_from_event_loop, SharedString};
use std::process::Command;
use std::{env, str};
use redox_log::{OutputBuilder, RedoxLogger};
use log::{error, info};

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

fn login_command(username: &str, launcher_cmd: &str, launcher_args: &[String]) -> Option<Command> {
    let sys_users = match AllUsers::authenticator(Config::default()) {
        Ok(users) => users,
        // Not maybe the best thing to do...
        Err(_) => {
            return None;
        }
    };

    match sys_users.get_by_name(username) {
        Some(user) => {
            let mut command = user.login_cmd(launcher_cmd);
            for arg in launcher_args.iter() {
                command.arg(arg);
            }
            Some(command)
        }
        None => None,
    }
}

fn authenticate(username: SharedString, password: SharedString) -> bool {
    let username = String::from(username);
    let password = String::from(password);

    let sys_users = match AllUsers::authenticator(Config::default()) {
        Ok(users) => users,
        // Not maybe the best thing to do...
        Err(_) => {
            return false;
        }
    };

    match sys_users.get_by_name(&username) {
        Some(user) => {
            if user.verify_passwd(password) {
                return true;
            }
            false
        }
        None => false,
    }
}

fn fullscreen(login_window: &LoginWindow) {
    // workaround for missing full screen window feature from both slint and orbital
    //set window size to display size, e.g. fullscreen window
    let (display_width, display_height) =
        orbclient::get_display_size().expect("Cannot query display size!");

    let login_window_weak = login_window.as_weak();
    let lww = login_window_weak.unwrap();
    let window = lww.window();
    window.set_size(slint::WindowSize::Physical(slint::PhysicalSize {
        height: display_height,
        width: display_width,
    }));
    window.set_position(slint::WindowPosition::Physical(slint::PhysicalPosition {
        x: 0,
        y: 0
    }));
    login_window.set_window_width(display_width as f32);
    login_window.set_window_height(display_height as f32);
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
     .with_process_name("orblogin".into())
     .enable();

    let mut args = env::args().skip(1);
    let launcher_cmd = args
        .next()
        .expect("orblogin: no window manager command provided!");
    let launcher_args: Vec<String> = args.collect();
    let users = normal_usernames();

    env::set_var("SLINT_FULLSCREEN", "1");

    let login_window = LoginWindow::new().expect("orblogin: cannot create LoginWindow!");
    login_window.on_authenticate(authenticate);

    fullscreen(&login_window);

    let login_window_weak = login_window.as_weak();
    login_window.on_exec_login_cmd(move || {
        let login_window_weak_copy = login_window_weak.clone();
        let launcher_cmd = launcher_cmd.clone();
        let launcher_args = launcher_args.clone();

        invoke_from_event_loop(move || {
            let login_window = login_window_weak_copy.unwrap();
            let is_authenticated: bool = !login_window.get_login_failed();

            if is_authenticated {
                // FIXME: currently orbclient has missing implementations in winit to hide the window.
                // On Redox OS, the login window still shows after calling hide()
                let slint_window = login_window.window();
                slint_window.set_size(slint::WindowSize::Physical(slint::PhysicalSize {
                    height: 0,
                    width: 0,
                }));

                // dequeue this window from the rendering loop
                login_window.hide().expect("orblogin: cannot hide LoginWindow!");

                let username = String::from(login_window.get_username_input());
                match login_command(&username, &launcher_cmd, &launcher_args) {
                    Some(mut login_cmd) => match login_cmd.spawn() {
                        Ok(mut child) => match child.wait() {
                            Ok(_) => (),
                            Err(error) => {
                                error!("failed to wait for '{launcher_cmd}' : {error}")
                            }
                        },
                        Err(error) => {
                            error!("failed to execute '{launcher_cmd}': {error}")
                        }
                    },
                    // there is a valid user without login command
                    None => info!("login completed without a command!"),
                }

                // all child process are exited, reset and display the login form again
                fullscreen(&login_window);
                login_window.set_reset_form(true);
                login_window.show().expect("orblogin: cannot show LoginWindow")
            }
        })
        .expect("orblogin: upgrade_in_event_loop() returned Err!");
    });

    // prefill the username input when there are only one unprivileged user
    if users.len() == 1 {
        let _prefilled_username = users
            .get(0)
            .map_or_else(|| String::new(), |user| user.clone());
        //login_window.set_username_input(SharedString::from(prefilled_username));
        // FIXME: workaround for cursor position not updating when the value is set by programmatically,
        for c in "user".chars() {
            login_window.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: c.into() });
        }
        login_window.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: "".into() });
    }

    login_window.run().expect("orblogin: cannot start LoginWindow!");
}
