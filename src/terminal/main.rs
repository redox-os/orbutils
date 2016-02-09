extern crate orbclient;

use orbclient::Color;

use std::ops::DerefMut;
use std::io::Read;
use std::process::{Command, Stdio};
use std::str;
use std::thread;

use window::ConsoleWindow;

mod window;

fn main() {
    let mut window = ConsoleWindow::new(-1, -1, 576, 400, "Terminal");

    loop {
        window.print("# ", Color::rgb(255, 255, 255));
        if let Some(line_original) = window.read() {
            let line = line_original.trim().to_string();
            if ! line.is_empty() {
                match Command::new("sh")
                        .arg("-c")
                        .arg(&line)
                        .stdout(Stdio::piped())
                        .spawn()
                {
                    Ok(process) => {
                        let mut output = String::new();
                        match process.stdout.unwrap().read_to_string(&mut output) {
                            Ok(_) => window.print(&output, Color::rgb(255, 255, 255)),
                            Err(err) => window.print(&format!("failed to get output from '{}': {}\n", line, err), Color::rgb(255, 0, 0))
                        }
                    },
                    Err(err) => {
                        window.print(&format!("failed to execute '{}': {}\n", line, err), Color::rgb(255, 0, 0));
                    }
                }
            }
        } else {
            break;
        }
    }
}
