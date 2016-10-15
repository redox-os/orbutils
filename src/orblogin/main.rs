#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;
extern crate orbtk;
extern crate userutils;

use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::str;
use std::sync::{Arc, Mutex};

use orbtk::{Button, Label, Placeable, Point, Rect, TextBox, Window};
use orbtk::callback::{Click, Enter};
use userutils::Passwd;

pub fn main() {
    loop {
        let user_lock = Arc::new(Mutex::new(String::new()));
        let pass_lock = Arc::new(Mutex::new(String::new()));

        {
            let (width, height) = orbclient::get_display_size().expect("launcher: failed to get display size");
            let mut window = Window::new(Rect::new((width as i32 - 576)/2, (height as i32 - 112)/2, 576, 112), "");

            Label::new()
                .text("Username")
                .position(0, 0)
                .size(576, 16)
                .place(&window);

            let user_text_box = TextBox::new()
                .position(0, 16)
                .size(576, 16)
                .on_enter(|_| {
                })
                .place(&window);

            Label::new()
                .text("Password")
                .position(0, 48)
                .size(576, 16)
                .place(&window);

            let pass_text_box = TextBox::new()
                .position(0, 64)
                .size(576, 16)
                .on_enter(|_| {
                })
                .place(&window);

            let user_login = user_lock.clone();
            let pass_login = pass_lock.clone();
            let window_login = &mut window as *mut Window;
            Button::new()
                .position(0, 96)
                .size(576, 16)
                .text("Login")
                .on_click(move |_button: &Button, _point: Point| {
                    println!("Login {}", user_text_box.text.get());
                    *user_login.lock().unwrap() = user_text_box.text.get();
                    *pass_login.lock().unwrap() = pass_text_box.text.get();
                    unsafe { (&mut *window_login).close(); }
                })
                .place(&window);

            window.exec();
        }

        let user = user_lock.lock().unwrap().clone();
        let pass = pass_lock.lock().unwrap().clone();
        if ! user.is_empty() {
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
                let password_hash = Passwd::encode(&pass);

                for line in passwd_string.lines() {
                    if let Ok(passwd) = Passwd::parse(line) {
                        if user == passwd.user && password_hash == passwd.hash {
                            passwd_option = Some(passwd);
                            break;
                        }
                    }
                }
            }

            if let Some(passwd) = passwd_option {
                let mut command = Command::new("launcher");

                command.uid(passwd.uid);
                command.gid(passwd.gid);

                command.current_dir(passwd.home);

                command.env("USER", &user);
                command.env("HOME", passwd.home);

                match command.spawn() {
                    Ok(mut child) => match child.wait() {
                        Ok(_status) => (), //println!("login: waited for {}: {:?}", sh, status.code()),
                        Err(err) => panic!("orblogin: failed to wait for '{}': {}", passwd.shell, err)
                    },
                    Err(err) => panic!("orblogin: failed to execute '{}': {}", passwd.shell, err)
                }
            }
        }
    }
}
