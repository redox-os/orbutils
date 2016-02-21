extern crate orbclient;

use orbclient::Color;

use std::env;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use window::{ConsoleEvent, ConsoleWindow};

mod window;

fn main() {
    let shell = env::args().nth(1).unwrap_or("sh".to_string());
    match Command::new(&shell)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    {
        Ok(process) => {
            let output_mutex = Arc::new(Mutex::new(Vec::new()));

            {
                let mut stdout = process.stdout.unwrap();
                let stdout_output_mutex = output_mutex.clone();
                thread::spawn(move || {
                    'stdout: loop {
                        let mut buf = [0; 4096];
                        match stdout.read(&mut buf) {
                            Ok(0) => break 'stdout,
                            Ok(count) => {
                                match stdout_output_mutex.lock() {
                                    Ok(mut stdout_output) => stdout_output.extend_from_slice(&buf[..count]),
                                    Err(_) => {
                                        println!("failed to lock stdout output mutex");
                                        break 'stdout;
                                    }
                                }
                            },
                            Err(err) => {
                                println!("failed to read stdout: {}", err);
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
                    'stderr: loop {
                        let mut buf = [0; 4096];
                        match stderr.read(&mut buf) {
                            Ok(0) => break 'stderr,
                            Ok(count) => {
                                match stderr_output_mutex.lock() {
                                    Ok(mut stderr_output) => stderr_output.extend_from_slice(&buf[..count]),
                                    Err(_) => {
                                        println!("failed to lock stderr output mutex");
                                        break 'stderr;
                                    }
                                }
                            },
                            Err(err) => {
                                println!("failed to read stderr: {}", err);
                                break 'stderr;
                            }
                        }
                    }
                });
            }

            let mut stdin = process.stdin.unwrap();
            let mut window = ConsoleWindow::new(-1, -1, 576, 400, "Terminal");
            'events: loop {
                match output_mutex.lock() {
                    Ok(mut output) => {
                        let mut string = String::new();
                        for byte in output.drain(..) {
                            string.push(byte as char);
                        }
                        window.print(&string, Color::rgb(255, 255, 255));
                    },
                    Err(_) => {
                        println!("failed to lock print output mutex");
                        break 'events;
                    }
                }

                for console_event in window.read() {
                    match console_event {
                        ConsoleEvent::Line(mut line) => {
                            line.push('\n');
                            match stdin.write(&line.as_bytes()) {
                                Ok(_) => (),
                                Err(err) => {
                                    println!("failed to write stdin: {}", err);
                                    break 'events;
                                }
                            }
                        },
                        ConsoleEvent::Quit => break 'events
                    }
                }

                thread::sleep_ms(30);
            }
        },
        Err(err) => println!("failed to execute '{}': {}\n", shell, err)
    }
}
