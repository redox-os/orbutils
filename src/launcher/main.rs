#![deny(warnings)]
#![feature(const_fn)]

extern crate orbclient;
extern crate orbimage;
extern crate orbfont;
extern crate syscall;

use std::env;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, ExitStatus};

use orbclient::{Color, EventOption, Window, K_ESC};
use orbimage::Image;
use orbfont::Font;

use package::Package;

pub mod package;

const BAR_COLOR: Color = Color::rgb(40, 45, 57);
const BAR_HIGHLIGHT_COLOR: Color = Color::rgb(80, 86, 102);
const TEXT_COLOR: Color = Color::rgb(204, 210, 224);
const TEXT_HIGHLIGHT_COLOR: Color = Color::rgb(235, 241, 255);

fn get_packages() -> Vec<Package> {
    let read_dir = Path::new("/ui/apps/").read_dir().expect("failed to read_dir on /ui/apps/");

    let mut entries = vec![];
    for dir in read_dir {
        let dir = match dir {
            Ok(x) => x,
            Err(_) => continue,
        };
        let file_name = dir.file_name().to_string_lossy().into_owned();
        if dir.file_type().expect("failed to get file_type").is_file() {
            entries.push(file_name);
        }
    }

    entries.sort();

    let mut packages: Vec<Package> = Vec::new();
    for entry in entries.iter() {
        packages.push(Package::from_path(&format!("/ui/apps/{}", entry)));
    }

    packages
}

fn draw(window: &mut Window, packages: &Vec<Package>, start: &Image, shutdown: &Image, selected: i32){
    let w = window.width();
    let h = window.height();
    window.set(BAR_COLOR);

    let mut x = 0;
    let mut i = 0;

    {
        let y = h as isize - start.height() as isize;

        if i == selected {
            window.rect(x as i32, y as i32,
                              start.width() as u32, start.height() as u32,
                              BAR_HIGHLIGHT_COLOR);
        }

        start.draw(window, x as i32, y as i32);

        x += start.width() as i32;
        i += 1;
    }

    for package in packages.iter() {
        let y = h as isize - package.icon.height() as isize;

        if i == selected {
            window.rect(x as i32, y as i32,
                              package.icon.width() as u32, package.icon.height() as u32,
                              BAR_HIGHLIGHT_COLOR);
        }

        package.icon.draw(window, x as i32, y as i32);

        x += package.icon.width() as i32;
        i += 1;
    }

    {
        x = w as i32 - shutdown.width() as i32;
        let y = h as isize - shutdown.height() as isize;

        if i == selected {
            window.rect(x as i32, y as i32,
                              shutdown.width() as u32, shutdown.height() as u32,
                              BAR_HIGHLIGHT_COLOR);
        }

        shutdown.draw(window, x as i32, y as i32);
    }

    window.sync();
}

fn draw_chooser(window: &mut Window, font: &Font, packages: &Vec<Package>, selected: i32){
    let w = window.width();

    window.set(BAR_COLOR);

    let mut y = 0;
    for (i, package) in packages.iter().enumerate() {
        if i as i32 == selected {
            window.rect(0, y, w, 32, BAR_HIGHLIGHT_COLOR);
        }

        package.icon.draw(window, 0, y);

        let mut c_x = 40;
        for c in package.name.chars() {
            font.render(&c.to_string(), 16.0).draw(window, c_x as i32, y + 8, if i as i32 == selected { TEXT_HIGHLIGHT_COLOR } else { TEXT_COLOR });
            c_x += 8;
        }

        y += 32;
    }

    window.sync();
}

fn bar_main() {
    let mut children = Vec::new();

    let packages = get_packages();

    let start = Image::from_path("/ui/icons/start.png").unwrap_or(Image::default());

    let shutdown = Image::from_path("/ui/icons/actions/system-log-out.png").unwrap_or(Image::default());

    let (width, height) = orbclient::get_display_size().expect("launcher: failed to get display size");
    let mut window = Window::new(0, height as i32 - 32, width, 32, "").expect("launcher: failed to open window");

    let mut selected = -1;
    let mut last_left_button = false;

    draw(&mut window, &packages, &start, &shutdown, selected);
    'running: loop {
        for event in window.events() {
            match event.to_option() {
                EventOption::Mouse(mouse_event) => {
                    let mut now_selected = -1;

                    {
                        let mut x = 0;
                        let mut i = 0;

                        {
                            let y = window.height() as i32 - start.height() as i32;
                            if mouse_event.y >= y && mouse_event.x >= x &&
                               mouse_event.x < x + start.width() as i32 {
                                   now_selected = i;
                            }
                            x += start.width() as i32;
                            i += 1;
                        }

                        for package in packages.iter() {
                            let y = window.height() as i32 - package.icon.height() as i32;
                            if mouse_event.y >= y && mouse_event.x >= x &&
                               mouse_event.x < x + package.icon.width() as i32 {
                                now_selected = i;
                            }
                            x += package.icon.width() as i32;
                            i += 1;
                        }

                        {
                            x = window.width() as i32 - shutdown.width() as i32;
                            let y = window.height() as i32 - shutdown.height() as i32;
                            if mouse_event.y >= y && mouse_event.x >= x &&
                               mouse_event.x < x + shutdown.width() as i32 {
                                   now_selected = i;
                            }
                        }
                    }

                    if now_selected != selected {
                        selected = now_selected;
                        draw(&mut window, &packages, &start, &shutdown, selected);
                    }

                    if ! mouse_event.left_button && last_left_button {
                        let mut i = 0;

                        if i == selected {
                            let start_h = packages.len() as u32 * 32;
                            let mut start_window = Window::new(0, height as i32 - 32 - start_h as i32, 400, start_h, "").unwrap();
                            let font = Font::find(None, None, None).unwrap();

                            let mut selected = -1;
                            let mut last_left_button = false;

                            draw_chooser(&mut start_window, &font, &packages, selected);
                            'start_choosing: loop {
                                for event in start_window.events() {
                                    match event.to_option() {
                                        EventOption::Mouse(mouse_event) => {
                                            let mut now_selected = -1;

                                            let mut y = 0;
                                            for (i, _package) in packages.iter().enumerate() {
                                                if mouse_event.y >= y && mouse_event.y < y + 32 {
                                                    now_selected = i as i32;
                                                }
                                                y += 32;
                                            }

                                            if now_selected != selected {
                                                selected = now_selected;
                                                draw_chooser(&mut start_window, &font, &packages, selected);
                                            }

                                            if ! mouse_event.left_button && last_left_button {
                                                let mut y = 0;
                                                for package in packages.iter() {
                                                    if mouse_event.y >= y && mouse_event.y < y + 32 {
                                                        match Command::new(&package.binary).spawn() {
                                                            Ok(child) => children.push(child),
                                                            Err(err) => println!("launcher: failed to launch {}: {}", package.binary, err)
                                                        }
                                                        break 'start_choosing;
                                                    }
                                                    y += 32;
                                                }
                                            }

                                            last_left_button = mouse_event.left_button;
                                        },
                                        EventOption::Key(key_event) => {
                                            match key_event.scancode {
                                                K_ESC => break 'start_choosing,
                                                _ => ()
                                            }
                                        },
                                        EventOption::Focus(focus_event) => if ! focus_event.focused {
                                            break 'start_choosing;
                                        },
                                        EventOption::Quit(_) => break 'start_choosing,
                                        _ => ()
                                    }
                                }
                            }
                        }
                        i += 1;

                        for package in packages.iter() {
                            if i == selected {
                                match Command::new(&package.binary).spawn() {
                                    Ok(child) => children.push(child),
                                    Err(err) => println!("launcher: failed to launch {}: {}", package.binary, err)
                                }
                            }
                            i += 1;
                        }

                        if i == selected {
                               break 'running;
                        }
                    }

                    last_left_button = mouse_event.left_button;
                },
                EventOption::Quit(_) => break 'running,
                _ => ()
            }
        }
    }

    for mut child in children {
        let pid = child.id();
        match child.kill() {
            Ok(()) => (),
            Err(err) => println!("launcher: failed to kill {}: {}", pid, err),
        }
        match child.wait() {
            Ok(status) => println!("launcher: {} exited with {}", pid, status),
            Err(err) => println!("launcher: failed to wait for {}: {}", pid, err),
        }
    }

    loop {
        let mut status = 0;
        let pid = syscall::waitpid(0, &mut status, syscall::WNOHANG).unwrap();
        if pid == 0 {
            break;
        } else {
            println!("launcher: reaping zombie {}: {}", pid, ExitStatus::from_raw(status as i32));
        }
    }
}

fn chooser_main(paths: env::Args) {
    for ref path in paths.skip(1) {
        let mut packages = get_packages();

        packages.retain(|package| -> bool {
            for accept in package.accepts.iter() {
                if (accept.starts_with('*') && path.ends_with(&accept[1 ..])) ||
                   (accept.ends_with('*') && path.starts_with(&accept[.. accept.len() - 1])) {
                    return true;
                }
            }
            false
        });

        if packages.len() > 1 {
            let mut window = Window::new(-1, -1, 400, packages.len() as u32 * 32, path).expect("launcher: failed to open window");
            let font = Font::find(None, None, None).expect("launcher: failed to open font");

            let mut selected = -1;
            let mut last_left_button = false;

            draw_chooser(&mut window, &font, &packages, selected);
            'choosing: loop {
                for event in window.events() {
                    match event.to_option() {
                        EventOption::Mouse(mouse_event) => {
                            let mut now_selected = -1;

                            let mut y = 0;
                            for (i, _package) in packages.iter().enumerate() {
                                if mouse_event.y >= y && mouse_event.y < y + 32 {
                                    now_selected = i as i32;
                                }
                                y += 32;
                            }

                            if now_selected != selected {
                                selected = now_selected;
                                draw_chooser(&mut window, &font, &packages, selected);
                            }

                            if ! mouse_event.left_button && last_left_button {
                                let mut y = 0;
                                for package in packages.iter() {
                                    if mouse_event.y >= y && mouse_event.y < y + 32 {
                                        if let Err(err) = Command::new(&package.binary).arg(path).spawn() {
                                            println!("launcher: failed to launch {}: {}", package.binary, err);
                                        }
                                        break 'choosing;
                                    }
                                    y += 32;
                                }
                            }

                            last_left_button = mouse_event.left_button;
                        },
                        EventOption::Quit(_) => break 'choosing,
                        _ => ()
                    }
                }
            }
        } else if let Some(package) = packages.get(0) {
            if let Err(err) = Command::new(&package.binary).arg(&path).spawn() {
                println!("launcher: failed to launch '{}': {}", package.binary, err);
            }
        } else {
            println!("launcher: no application found for '{}'", path);
        }
    }
}

fn main() {
    let paths = env::args();
    if paths.len() > 1 {
        chooser_main(paths);
    } else {
        bar_main();
    }
}
