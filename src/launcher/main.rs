#![deny(warnings)]
#![feature(const_fn)]

extern crate event;
extern crate orbclient;
extern crate orbimage;
extern crate orbfont;
extern crate syscall;

use std::{env, io, mem};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::rc::Rc;

use event::EventQueue;
use orbclient::{EventOption, Renderer, Window, WindowFlag, K_ESC};
use orbimage::Image;
use orbfont::Font;
use syscall::data::TimeSpec;
use syscall::flag::{CLOCK_MONOTONIC, CLOCK_REALTIME};

use package::Package;
use theme::{BAR_COLOR, BAR_HIGHLIGHT_COLOR, TEXT_COLOR, TEXT_HIGHLIGHT_COLOR};

mod package;
mod theme;

pub const ICON_SIZE: i32 = 48;
pub const ICON_SMALL_SIZE: i32 = 32;

#[cfg(target_os = "redox")]
static UI_PATH: &'static str = "/ui";

#[cfg(not(target_os = "redox"))]
static UI_PATH: &'static str = "ui";

#[cfg(not(target_os = "redox"))]
fn wait(status: &mut i32) -> usize {
    extern crate libc;

    use std::io::Error;

    let pid = unsafe { libc::waitpid(0, status as *mut i32, libc::WNOHANG) };
    if pid < 0 {
        panic!("waitpid failed: {}", Error::last_os_error());
    }
    pid as usize
}

#[cfg(target_os = "redox")]
fn wait(status: &mut usize) -> usize {
    extern crate syscall;

    syscall::waitpid(0, status, syscall::WNOHANG).unwrap()
}

fn load_icon(path: &str) -> Image {
    let icon = Image::from_path(path).unwrap_or(Image::default());
    if icon.width() == ICON_SIZE as u32 && icon.height() == ICON_SIZE as u32 {
        icon
    } else {
        icon.resize(ICON_SIZE as u32, ICON_SIZE as u32, orbimage::ResizeType::Lanczos3).unwrap()
    }
}

fn load_icon_small(path: &str) -> Image {
    let icon = Image::from_path(path).unwrap_or(Image::default());
    if icon.width() == ICON_SMALL_SIZE as u32 && icon.height() == ICON_SMALL_SIZE as u32 {
        icon
    } else {
        icon.resize(ICON_SMALL_SIZE as u32, ICON_SMALL_SIZE as u32, orbimage::ResizeType::Lanczos3).unwrap()
    }
}

fn get_packages() -> Vec<Package> {
    let read_dir = Path::new(&format!("{}/apps/", UI_PATH)).read_dir().expect("failed to read apps directory");

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
        packages.push(Package::from_path(&format!("{}/apps/{}", UI_PATH, entry)));
    }

    packages
}

fn draw_chooser(window: &mut Window, font: &Font, packages: &Vec<Package>, selected: i32){
    let w = window.width();

    window.set(BAR_COLOR);

    let mut y = 0;
    for (i, package) in packages.iter().enumerate() {
        if i as i32 == selected {
            window.rect(0, y, w, ICON_SMALL_SIZE as u32, BAR_HIGHLIGHT_COLOR);
        }

        package.icon_small.draw(window, 0, y);

        font.render(&package.name, 16.0).draw(window, ICON_SMALL_SIZE + 8, y + 8, if i as i32 == selected { TEXT_HIGHLIGHT_COLOR } else { TEXT_COLOR });

        y += ICON_SMALL_SIZE;
    }

    window.sync();
}

struct Bar {
    children: Vec<Child>,
    packages: Vec<Package>,
    start: Image,
    start_packages: Vec<Package>,
    font: Font,
    width: u32,
    height: u32,
    window: Window,
    selected: i32,
    time: String,
}

impl Bar {
    fn new() -> Bar {
        let packages = get_packages();

        let mut logout_package = Package::new();
        logout_package.name = "Logout".to_string();
        logout_package.icon = load_icon(&format!("{}/icons/actions/system-log-out.png", UI_PATH));
        logout_package.icon_small = load_icon_small(&format!("{}/icons/actions/system-log-out.png", UI_PATH));
        logout_package.binary = "exit".to_string();

        let mut start_packages = packages.clone();
        start_packages.push(logout_package);

        let (width, height) = orbclient::get_display_size().expect("launcher: failed to get display size");

        Bar {
            children: Vec::new(),
            packages: packages,
            start: load_icon(&format!("{}/icons/places/start-here.png", UI_PATH)),
            start_packages: start_packages,
            font: Font::find(Some("Sans"), None, None).unwrap(),
            width: width,
            height: height,
            window: Window::new_flags(
                0, height as i32 - ICON_SIZE, width, ICON_SIZE as u32, "", &[WindowFlag::Async]
            ).expect("launcher: failed to open window"),
            selected: -1,
            time: String::new()
        }
    }

    fn update_time(&mut self) {
        let mut time = TimeSpec::default();
        syscall::clock_gettime(CLOCK_REALTIME, &mut time).expect("launcher: failed to read time");

        let ts = time.tv_sec;
        let s = ts%86400;
        let h = s/3600;
        let m = s/60%60;
        self.time = format!("{:>02}:{:>02}", h, m)
    }

    fn draw(&mut self) {
        self.window.set(BAR_COLOR);

        let mut x = 0;
        let mut y = 0;
        let mut i = 0;

        {
            if i == self.selected {
                self.window.rect(x as i32, y as i32,
                                  self.start.width() as u32, self.start.height() as u32,
                                  BAR_HIGHLIGHT_COLOR);
            }

            self.start.draw(&mut self.window, x as i32, y as i32);

            x += self.start.width() as i32;
            i += 1;
        }

        for package in self.packages.iter() {
            if i == self.selected {
                self.window.rect(x as i32, y as i32,
                                  package.icon.width() as u32, package.icon.height() as u32,
                                  BAR_HIGHLIGHT_COLOR);
            }

            package.icon.draw(&mut self.window, x as i32, y as i32);

            x += package.icon.width() as i32;
            i += 1;
        }

        let text = self.font.render(&self.time, 32.0);
        x = self.width as i32 - text.width() as i32 - 8;
        y = (ICON_SIZE - text.height() as i32)/2;
        text.draw(&mut self.window, x, y, TEXT_HIGHLIGHT_COLOR);

        self.window.sync();
    }
}

fn bar_main() {
    let bar = Rc::new(RefCell::new(Bar::new()));

    let mut event_queue = EventQueue::<()>::new().expect("launcher: failed to create event queue");

    let mut time_file = File::open(&format!("time:{}", CLOCK_MONOTONIC)).expect("launcher: failed to open time");
    let bar_time = bar.clone();
    event_queue.add(time_file.as_raw_fd(), move |_| -> io::Result<Option<()>> {
        let mut time = TimeSpec::default();
        if time_file.read(&mut time)? >= mem::size_of::<TimeSpec>() {
            let mut bar = bar_time.borrow_mut();

            bar.update_time();
            bar.draw();

            time.tv_sec += 1;
            time.tv_nsec = 0;
            time_file.write(&time)?;
        }

        Ok(None)
    }).expect("launcher: failed to poll time");

    let bar_window = bar.clone();
    let mut mouse_x = 0;
    let mut mouse_y = 0;
    let mut mouse_left = false;
    let mut last_mouse_left = false;
    event_queue.add(bar.borrow().window.as_raw_fd(), move |_| -> io::Result<Option<()>> {
        let mut bar = bar_window.borrow_mut();

        for event in bar.window.events() {
            let redraw = match event.to_option() {
                EventOption::Mouse(mouse_event) => {
                    mouse_x = mouse_event.x;
                    mouse_y = mouse_event.y;
                    true
                },
                EventOption::Button(button_event) => {
                    mouse_left = button_event.left;
                    true
                },
                EventOption::Screen(screen_event) => {
                    bar.width = screen_event.width;
                    bar.height = screen_event.height;
                    bar.window.set_pos(0, screen_event.height as i32 - ICON_SIZE);
                    bar.window.set_size(screen_event.width, ICON_SIZE as u32);
                    true
                },
                EventOption::Quit(_) => return Ok(Some(())),
                _ => false
            };

            if redraw {
                let mut now_selected = -1;

                {
                    let mut x = 0;
                    let y = 0;
                    let mut i = 0;

                    {
                        if mouse_y >= y && mouse_x >= x &&
                           mouse_x < x + bar.start.width() as i32 {
                               now_selected = i;
                        }
                        x += bar.start.width() as i32;
                        i += 1;
                    }

                    for package in bar.packages.iter() {
                        if mouse_y >= y && mouse_x >= x &&
                           mouse_x < x + package.icon.width() as i32 {
                            now_selected = i;
                        }
                        x += package.icon.width() as i32;
                        i += 1;
                    }
                }

                if now_selected != bar.selected {
                    bar.selected = now_selected;
                    bar.draw()
                }

                if ! mouse_left && last_mouse_left {
                    let mut i = 0;

                    if i == bar.selected {
                        let start_h = bar.start_packages.len() as u32 * ICON_SMALL_SIZE as u32;
                        let mut start_window = Window::new(0, bar.height as i32 - ICON_SIZE - start_h as i32, 320, start_h, "").unwrap();

                        let mut selected = -1;
                        let mut mouse_y = 0;
                        let mut mouse_left = false;
                        let mut last_mouse_left = false;

                        draw_chooser(&mut start_window, &bar.font, &bar.start_packages, selected);
                        'start_choosing: loop {
                            for event in start_window.events() {
                                let redraw = match event.to_option() {
                                    EventOption::Mouse(mouse_event) => {
                                        mouse_y = mouse_event.y;
                                        true
                                    },
                                    EventOption::Button(button_event) => {
                                        mouse_left = button_event.left;
                                        true
                                    },
                                    EventOption::Key(key_event) => {
                                        match key_event.scancode {
                                            K_ESC => break 'start_choosing,
                                            _ => false
                                        }
                                    },
                                    EventOption::Focus(focus_event) => if ! focus_event.focused {
                                        break 'start_choosing;
                                    } else {
                                        false
                                    },
                                    EventOption::Quit(_) => break 'start_choosing,
                                    _ => false
                                };

                                if redraw {
                                    let mut now_selected = -1;

                                    let mut y = 0;
                                    for (j, _package) in bar.start_packages.iter().enumerate() {
                                        if mouse_y >= y && mouse_y < y + ICON_SMALL_SIZE {
                                            now_selected = j as i32;
                                        }
                                        y += ICON_SMALL_SIZE;
                                    }

                                    if now_selected != selected {
                                        selected = now_selected;
                                        draw_chooser(&mut start_window, &bar.font, &bar.start_packages, selected);
                                    }

                                    if ! mouse_left && last_mouse_left {
                                        let mut y = 0;
                                        for package_i in 0..bar.start_packages.len() {
                                            if mouse_y >= y && mouse_y < y + ICON_SMALL_SIZE {
                                                if bar.start_packages[package_i].binary == "exit" {
                                                    return Ok(Some(()));
                                                } else {
                                                    match Command::new(&bar.start_packages[package_i].binary).spawn() {
                                                        Ok(child) => bar.children.push(child),
                                                        Err(err) => println!("launcher: failed to launch {}: {}", bar.start_packages[package_i].binary, err)
                                                    }
                                                }
                                                break 'start_choosing;
                                            }
                                            y += ICON_SMALL_SIZE;
                                        }
                                    }

                                    last_mouse_left = mouse_left;
                                }
                            }
                        }
                    }
                    i += 1;

                    for package_i in 0..bar.packages.len() {
                        if i == bar.selected {
                            match Command::new(&bar.packages[package_i].binary).spawn() {
                                Ok(child) => bar.children.push(child),
                                Err(err) => println!("launcher: failed to launch {}: {}", bar.packages[package_i].binary, err)
                            }
                        }
                        i += 1;
                    }
                }

                last_mouse_left = mouse_left;
            }
        }

        Ok(None)
    }).expect("launcher: failed to poll window events");

    event_queue.trigger_all(0).expect("launcher: failed to trigger events");

    event_queue.run().expect("launcher: failed to run event loop");

    for mut child in bar.borrow_mut().children.iter_mut() {
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
        let pid = wait(&mut status);
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
            let mut window = Window::new(-1, -1, 320, packages.len() as u32 * ICON_SMALL_SIZE as u32, path).expect("launcher: failed to open window");
            let font = Font::find(Some("Sans"), None, None).expect("launcher: failed to open font");

            let mut selected = -1;
            let mut mouse_y = 0;
            let mut mouse_left = false;
            let mut last_mouse_left = false;

            draw_chooser(&mut window, &font, &packages, selected);
            'choosing: loop {
                for event in window.events() {
                    let redraw = match event.to_option() {
                        EventOption::Mouse(mouse_event) => {
                            mouse_y = mouse_event.y;
                            true
                        },
                        EventOption::Button(button_event) => {
                            mouse_left = button_event.left;
                            true
                        },
                        EventOption::Quit(_) => break 'choosing,
                        _ => false
                    };

                    if redraw {
                        let mut now_selected = -1;

                        let mut y = 0;
                        for (i, _package) in packages.iter().enumerate() {
                            if mouse_y >= y && mouse_y < y + ICON_SIZE {
                                now_selected = i as i32;
                            }
                            y += ICON_SMALL_SIZE;
                        }

                        if now_selected != selected {
                            selected = now_selected;
                            draw_chooser(&mut window, &font, &packages, selected);
                        }

                        if ! mouse_left && last_mouse_left {
                            let mut y = 0;
                            for package in packages.iter() {
                                if mouse_y >= y && mouse_y < y + ICON_SMALL_SIZE {
                                    if let Err(err) = Command::new(&package.binary).arg(path).spawn() {
                                        println!("launcher: failed to launch {}: {}", package.binary, err);
                                    }
                                    break 'choosing;
                                }
                                y += ICON_SMALL_SIZE;
                            }
                        }

                        last_mouse_left = mouse_left;
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
