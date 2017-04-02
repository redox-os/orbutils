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

use orbclient::WindowFlag;
use orbtk::{Button, Label, Point, Rect, TextBox, Window};
use orbtk::traits::{Click, Enter, Place, Text};
use userutils::Passwd;

pub fn main() {
    let mut args = env::args().skip(1);

    let launcher_cmd = args.next().expect("orblogin: no window manager command");
    let launcher_args: Vec<String> = args.collect();

    loop {
        let user_lock = Arc::new(Mutex::new(String::new()));
        let pass_lock = Arc::new(Mutex::new(String::new()));

        {
            let mut issue_string = String::new();
            if let Ok(mut issue_file) = File::open("/etc/issue") {
                let _ = issue_file.read_to_string(&mut issue_string);
            }

            const MARGIN: u32 = 8;
            const TEXT_HEIGHT: u32 = 16;
            const INPUT_HEIGHT: u32 = 28;

            let issue = issue_string.trim();
            let issue_height = issue.lines().count() as u32 * TEXT_HEIGHT;

            const WINDOW_WIDTH: u32 = 576;
            let window_height: u32 = 148 + issue_height + if issue_height > 0 { 20 } else { 0 };


            let (width, height) = orbclient::get_display_size().expect("launcher: failed to get display size");
            let mut window = Window::new_flags(
                Rect::new((width as i32 - WINDOW_WIDTH as i32)/2, (height as i32 - window_height as i32)/2, WINDOW_WIDTH, window_height),
                "Orbital Login",
                &[WindowFlag::Unclosable]
                );

            let mut y = 8;

            if ! issue.is_empty() {
                let label = Label::new();
                label.text(issue)
                    .text_offset(6, 6)
                    .position(MARGIN as i32, y)
                    .size(WINDOW_WIDTH - MARGIN*2, issue_height + 12);
                //TODO: Put inset color into theme
                label.bg.set(orbclient::Color { data: 0xFFEFF1F2 });
                label.border.set(true);
                label.border_radius.set(2);
                window.add(&label);

                y += issue_height as i32 + 20;
            }

            let label = Label::new();
            label.text("Username")
                .position(MARGIN as i32, y)
                .size(WINDOW_WIDTH - MARGIN*2, TEXT_HEIGHT);
            window.add(&label);
            y += TEXT_HEIGHT as i32;

            let user_text_box = TextBox::new();
            user_text_box.position(MARGIN as i32, y)
                .size(WINDOW_WIDTH - MARGIN*2, INPUT_HEIGHT)
                .text_offset(6, 6)
                .grab_focus(true);
            user_text_box.border_radius.set(2);
            window.add(&user_text_box);
            y += INPUT_HEIGHT as i32;

            y += 8;

            let label = Label::new();
            label.text("Password")
                .position(MARGIN as i32, y)
                .size(WINDOW_WIDTH - MARGIN*2, TEXT_HEIGHT);
            window.add(&label);
            y += TEXT_HEIGHT as i32;

            let pass_text_box = TextBox::new();
            pass_text_box.position(MARGIN as i32, y)
                .size(WINDOW_WIDTH - MARGIN*2, INPUT_HEIGHT)
                .text_offset(6, 6)
                .mask_char(Some('*'));
            pass_text_box.border_radius.set(2);
            window.add(&pass_text_box);
            y += INPUT_HEIGHT as i32;

            // Pressing enter in user text box will transfer focus to password text box
            {
                let pass_text_box = pass_text_box.clone();
                user_text_box.on_enter(move |_| {
                    pass_text_box.grab_focus.set(true);
                });
            }

            // Pressing enter in password text box will try to login
            {
                let user_lock = user_lock.clone();
                let pass_lock = pass_lock.clone();
                let user_text_box = user_text_box.clone();
                let window_login = &mut window as *mut Window;
                pass_text_box.on_enter(move |me: &TextBox| {
                    println!("Login {}", user_text_box.text.get());
                    *user_lock.lock().unwrap() = user_text_box.text.get();
                    *pass_lock.lock().unwrap() = me.text.get();
                    unsafe { (&mut *window_login).close(); }
                });
            }

            y += 8;

            // Add a login button
            {
                let user_lock = user_lock.clone();
                let pass_lock = pass_lock.clone();
                let window_login = &mut window as *mut Window;
                let button = Button::new();
                button.position(MARGIN as i32, y)
                    .size(55, INPUT_HEIGHT)
                    .text("Login")
                    .text_offset(6, 6)
                    .on_click(move |_button: &Button, _point: Point| {
                        *user_lock.lock().unwrap() = user_text_box.text.get();
                        *pass_lock.lock().unwrap() = pass_text_box.text.get();
                        unsafe { (&mut *window_login).close(); }
                    });
                window.add(&button);
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
                command.env("UID", format!("{}", passwd.uid));
                command.env("GROUPS", format!("{}", passwd.gid));
                command.env("HOME", passwd.home);
                command.env("SHELL", passwd.shell);

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
