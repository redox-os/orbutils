extern crate orbclient;

use orbclient::Color;

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use window::ConsoleWindow;

mod window;

fn main() {
    let mut window = ConsoleWindow::new(-1, -1, 576, 400, "Terminal");

    match Command::new("sh")
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
                            Ok(count) => match stdout_output_mutex.lock() {
                                Ok(mut stdout_output) => stdout_output.extend_from_slice(&buf[..count]),
                                Err(err) => {
                                    println!("failed to lock stdout output mutex: {}", err);
                                    break 'stdout;
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
                            Ok(count) => match stderr_output_mutex.lock() {
                                Ok(mut stderr_output) => stderr_output.extend_from_slice(&buf[..count]),
                                Err(err) => {
                                    println!("failed to lock stderr output mutex: {}", err);
                                    break 'stderr;
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
            'events: loop {
                match output_mutex.lock() {
                    Ok(mut output) => {
                        let mut string = String::new();
                        for byte in output.drain(..) {
                            string.push(byte as char);
                        }
                        window.print(&string, Color::rgb(255, 255, 255));
                    },
                    Err(err) => {
                        println!("failed to lock print output mutex: {}", err);
                        break 'events;
                    }
                }

                window.print("# ", Color::rgb(255, 255, 255));
                if let Some(mut line) = window.read() {
                    line.push('\n');
                    match stdin.write(&line.as_bytes()) {
                        Ok(_) => (),
                        Err(err) => {
                            println!("failed to write stdin: {}", err);
                            break 'events;
                        }
                    }
                } else {
                    break 'events;
                }

                thread::sleep_ms(30);
            }
        },
        Err(err) => {
            window.print(&format!("failed to execute shell: {}\n", err), Color::rgb(255, 0, 0));
        }
    }
}
