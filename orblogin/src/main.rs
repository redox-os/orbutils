//#![deny(warnings)]

#[allow(clippy::all)]
mod generated_code {
    slint::include_modules!();
}

pub use generated_code::*;
use redox_users::{All, AllUsers, Config};
use slint::{invoke_from_event_loop, SharedString};
use std::process::Command;
use std::{env, str};

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

fn main() {
    let mut args = env::args().skip(1);

    let launcher_cmd = args
        .next()
        .expect("orblogin: no window manager command provided!");
    let launcher_args: Vec<String> = args.collect();
    let users = normal_usernames();

    let login_window = LoginWindow::new().expect("Cannot create LoginWindow!");
    login_window.on_authenticate(authenticate);

    let login_window_weak = login_window.as_weak();
    login_window.on_exec_login_cmd(move || {
        let login_window_weak_copy = login_window_weak.clone();
        let launcher_cmd = launcher_cmd.clone();
        let launcher_args = launcher_args.clone();

        invoke_from_event_loop(move || {
            let login_window = login_window_weak_copy.unwrap();
            let is_authenticated: bool = !login_window.get_login_failed();

            if is_authenticated {
                // dequeue this window from the rendering loop
                login_window.hide().expect("Cannot hide LoginWindow!");

                let username = String::from(login_window.get_username_input());
                match login_command(&username, &launcher_cmd, &launcher_args) {
                    Some(mut login_cmd) => match login_cmd.spawn() {
                        Ok(mut child) => match child.wait() {
                            Ok(_) => (),
                            Err(error) => {
                                eprintln!("orblogin: failed to wait for '{launcher_cmd}' : {error}")
                            }
                        },
                        Err(error) => {
                            eprintln!("orblogin: failed to execute '{launcher_cmd}': {error}")
                        }
                    },
                    // there is a valid user without login command
                    None => println!("login completed without a command!"),
                }
            }
        })
        .expect("orblogin: upgrade_in_event_loop() returned Err!");
    });

    // prefill the username input when there are only one unprivileged user
    if users.len() == 1 {
        let prefilled_username = users
            .get(0)
            .map_or_else(|| String::new(), |user| user.clone());
        login_window.set_username_input(SharedString::from(prefilled_username));
    }

    login_window.run().expect("Cannot start LoginWindow!");
}
