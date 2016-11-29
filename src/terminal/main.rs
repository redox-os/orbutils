#![deny(warnings)]
#![feature(asm)]
#![feature(const_fn)]

extern crate orbclient;
extern crate syscall;

use orbclient::event;

use std::{env, str};
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use std::process::{Command, Stdio};

use console::Console;
use getpty::getpty;

mod console;
mod getpty;

fn main() {
    let shell = env::args().nth(1).unwrap_or("sh".to_string());

    let (master_fd, tty_path) = getpty();

    let slave_stdin = File::open(&tty_path).unwrap();
    let slave_stdout = File::create(&tty_path).unwrap();
    let slave_stderr = File::create(&tty_path).unwrap();

    let width = 800;
    let height = 600;

    env::set_var("COLUMNS", format!("{}", width / 8));
    env::set_var("LINES", format!("{}", height / 16));
    env::set_var("TTY", format!("{}", tty_path.display()));

    match unsafe {
        Command::new(&shell)
            .stdin(Stdio::from_raw_fd(slave_stdin.into_raw_fd()))
            .stdout(Stdio::from_raw_fd(slave_stdout.into_raw_fd()))
            .stderr(Stdio::from_raw_fd(slave_stderr.into_raw_fd()))
            .spawn()
    } {
        Ok(mut process) => {
            {
                let mut console = Console::new(width, height);

                let mut event_file = File::open("event:").expect("terminal: failed to open event file");

                let window_fd = console.window.as_raw_fd();
                syscall::fevent(window_fd, syscall::flag::EVENT_READ).expect("terminal: failed to fevent console window");

                let mut master = unsafe { File::from_raw_fd(master_fd) };
                syscall::fevent(master_fd, syscall::flag::EVENT_READ).expect("terminal: failed to fevent master PTY");

                let mut handle_event = |event_id: usize, event_count: usize| -> bool {
                    if event_id == window_fd {
                        for event in console.window.events() {
                            if event.code == event::EVENT_QUIT {
                                return false;
                            }

                            console.input(&event);
                        }

                        if ! console.input.is_empty()  {
                            if let Err(err) = master.write(&console.input) {
                                let term_stderr = io::stderr();
                                let mut term_stderr = term_stderr.lock();

                                let _ = term_stderr.write(b"failed to write stdin: ");
                                let _ = term_stderr.write(err.description().as_bytes());
                                let _ = term_stderr.write(b"\n");
                                return false;
                            }
                            console.input.clear();
                        }
                    } else if event_id == master_fd {
                        let mut packet = [0; 4096];
                        let count = master.read(&mut packet).expect("terminal: failed to read master PTY");
                        if count == 0 {
                            if event_count == 0 {
                                return false;
                            }
                        } else {
                            console.write(&packet[1..count], true).expect("terminal: failed to write to console");

                            if packet[0] & 1 == 1 {
                                console.redraw();
                            }
                        }
                    } else {
                        println!("Unknown event {}", event_id);
                    }

                    true
                };

                handle_event(window_fd, 0);
                handle_event(master_fd, 0);

                'events: loop {
                    let mut sys_event = syscall::Event::default();
                    event_file.read(&mut sys_event).expect("terminal: failed to read event file");
                    if ! handle_event(sys_event.id, sys_event.data) {
                        break 'events;
                    }
                }
            }

            process.wait().expect("terminal: failed to wait on shell");
        },
        Err(err) => {
            let term_stderr = io::stderr();
            let mut term_stderr = term_stderr.lock();
            let _ = term_stderr.write(b"failed to execute '");
            let _ = term_stderr.write(shell.as_bytes());
            let _ = term_stderr.write(b"': ");
            let _ = term_stderr.write(err.description().as_bytes());
            let _ = term_stderr.write(b"\n");
        }
    }
}
