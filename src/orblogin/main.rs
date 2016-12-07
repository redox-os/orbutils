#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;
extern crate orbtk;
extern crate userutils;

use std::{env, str};
use std::fs::File;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};

use orbtk::{Button, Label, Placeable, Point, Rect, TextBox, Window};
use orbtk::callback::{Click, Enter};
use userutils::Passwd;

pub fn main() {
    let mut args = env::args().skip(1);

    let launcher_cmd = args.next().expect("orblogin: no window manager command");
    let launcher_args: Vec<String> = args.collect();

    loop {
        let user_lock = Arc::new(Mutex::new(String::new()));
        let pass_lock = Arc::new(Mutex::new(String::new()));

        {
            let mut issue = String::new();
            if let Ok(mut issue_file) = File::open("/etc/issue") {
                let _ = issue_file.read_to_string(&mut issue);
            }

            let window_height = 80 + issue.lines().count() as u32 * 16;
            let (width, height) = orbclient::get_display_size().expect("launcher: failed to get display size");
            let mut window = Window::new(Rect::new((width as i32 - 576)/2, (height as i32 - window_height as i32)/2, 576, window_height), "Orbital Login");

            let mut y = 0;
            for line in issue.lines() {
                Label::new()
                    .text(line)
                    .position(0, y)
                    .size(576, 16)
                    .place(&window);
                y += 16;
            }


            Label::new()
                .text("Username")
                .position(0, y)
                .size(576, 16)
                .place(&window);
            y += 16;

            let user_text_box = TextBox::new()
                .position(0, y)
                .size(576, 16)
                .grab_focus(true)
                .on_enter(|_| {
                })
                .place(&window);
            y += 16;

            Label::new()
                .text("Password")
                .position(0, y)
                .size(576, 16)
                .place(&window);
            y += 16;

            let pass_text_box = TextBox::new()
                .position(0, y)
                .size(576, 16)
                .mask_char(Some('*'))
                .place(&window);
            y += 16;

            // Pressing enter in user text box will transfer focus to password text box
            {
                let pass_text_box = pass_text_box.clone();
                *user_text_box.on_enter.borrow_mut() = Some(Arc::new(move |_| {
                    pass_text_box.grab_focus.set(true);
                }));
            }

            // Pressing enter in password text box will try to login
            {
                let user_lock = user_lock.clone();
                let pass_lock = pass_lock.clone();
                let user_text_box = user_text_box.clone();
                let window_login = &mut window as *mut Window;
                *pass_text_box.on_enter.borrow_mut() = Some(Arc::new(move |me: &TextBox| {
                    println!("Login {}", user_text_box.text.get());
                    *user_lock.lock().unwrap() = user_text_box.text.get();
                    *pass_lock.lock().unwrap() = me.text.get();
                    unsafe { (&mut *window_login).close(); }
                }));
            }

            // Add a login button
            {
                let user_lock = user_lock.clone();
                let pass_lock = pass_lock.clone();
                let window_login = &mut window as *mut Window;
                Button::new()
                    .position(0, y)
                    .size(576, 16)
                    .text("Login")
                    .on_click(move |_button: &Button, _point: Point| {
                        println!("Login {}", user_text_box.text.get());
                        *user_lock.lock().unwrap() = user_text_box.text.get();
                        *pass_lock.lock().unwrap() = pass_text_box.text.get();
                        unsafe { (&mut *window_login).close(); }
                    })
                    .place(&window);
            }

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
