#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;

use orbclient::event;

use std::env;
use std::io::{Read, Write, self};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::error::Error;

use console::Console;

mod console;

fn main() {
    let shell = env::args().nth(1).unwrap_or("sh".to_string());

    let width = 640;
    let height = 480;

    env::set_var("COLUMNS", format!("{}", width/8));
    env::set_var("LINES", format!("{}", height/16));
    match Command::new(&shell).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(process) => {
            let output_mutex = Arc::new(Mutex::new(Vec::new()));

            {
                let mut stdout = process.stdout.unwrap();
                let stdout_output_mutex = output_mutex.clone();
                thread::spawn(move || {
                    let term_stderr = io::stderr();
                    let mut term_stderr = term_stderr.lock();
                    'stdout: loop {
                        let mut buf = [0; 4096];
                        match stdout.read(&mut buf) {
                            Ok(0) => break 'stdout,
                            Ok(count) => {
                                if let Ok(mut stdout_output) = stdout_output_mutex.lock() {
                                    stdout_output.extend_from_slice(&buf[..count]);
                                } else {
                                    let _ = term_stderr.write(b"failed to lock stdout output mutex.\n");
                                    break 'stdout;
                                }
                            },
                            Err(err) => {
                                let _ = term_stderr.write(b"failed to read stdout: ");
                                let _ = term_stderr.write(err.description().as_bytes());
                                let _ = term_stderr.write(b"\n");
                                break 'stdout;
                            }
                        }
                    }
                });
            }

            {
                let mut stderr = process.stderr.unwrap();
                let stderr_output_mutex = output_mutex.clone();
                thread::spawn(move || {
                    let mut term_stderr = io::stderr();
                    'stderr: loop {
                        let mut buf = [0; 4096];
                        match stderr.read(&mut buf) {
                            Ok(0) => break 'stderr,
                            Ok(count) => {
                                match stderr_output_mutex.lock() {
                                    Ok(mut stderr_output) => stderr_output.extend_from_slice(&buf[..count]),
                                    Err(_) => {
                                        let _ = term_stderr.write(b"failed to lock stderr output mutex.\n");
                                        break 'stderr;
                                    }
                                }
                            },
                            Err(err) => {
                                let _ = term_stderr.write(b"failed to read stderr: ");
                                let _ = term_stderr.write(err.description().as_bytes());
                                let _ = term_stderr.write(b"\n");
                                break 'stderr;
                            }
                        }
                    }
                });
            }

            let mut stdin = process.stdin.unwrap();
            let mut console = Console::new(width, height);
            'events: loop {
                match output_mutex.lock() {
                    Ok(mut output) => {
                        if !output.is_empty() {
                            console.write(&output);
                            output.clear();
                        }
                    },
                    Err(_) => {
                        let term_stderr = io::stderr();
                        let mut term_stderr = term_stderr.lock();
                        let _ = term_stderr.write(b"failed to lock stdout mutex.\n");
                        break 'events;
                    }
                }

                for event in console.window.events() {
                    if event.code == event::EVENT_QUIT {
                        break 'events;
                    }

                    if let Some(line) = console.event(event) {
                        if let Err(err) = stdin.write(&line.as_bytes()) {
                            let term_stderr = io::stderr();
                            let mut term_stderr = term_stderr.lock();

                            let _ = term_stderr.write(b"failed to write stdin: ");
                            let _ = term_stderr.write(err.description().as_bytes());
                            let _ = term_stderr.write(b"\n");
                            break 'events;
                        }
                    }
                }

                thread::sleep_ms(1);
            }
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
