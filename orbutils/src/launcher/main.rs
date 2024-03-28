extern crate event;
extern crate orbclient;
extern crate orbimage;
extern crate orbfont;
extern crate redox_log;
extern crate libredox;
extern crate log;

use event::{user_data, EventQueue};
use libredox::data::TimeSpec;
use libredox::flag;
use redox_log::{OutputBuilder, RedoxLogger};
use log::{debug, error, info};
use std::{env, io, mem};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicIsize, Ordering};

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag, K_ESC};
use orbimage::Image;
use orbfont::Font;

use package::Package;
use theme::{BAR_COLOR, BAR_HIGHLIGHT_COLOR, TEXT_COLOR, TEXT_HIGHLIGHT_COLOR};

mod package;
mod theme;

static SCALE: AtomicIsize = AtomicIsize::new(1);

fn chooser_width() -> u32 {
    200 * SCALE.load(Ordering::Relaxed) as u32
}

fn font_size() -> i32 {
    16 * SCALE.load(Ordering::Relaxed) as i32
}

fn icon_size() -> i32 {
    48 * SCALE.load(Ordering::Relaxed) as i32
}

fn icon_small_size() -> i32 {
    32 * SCALE.load(Ordering::Relaxed) as i32
}

#[cfg(target_os = "redox")]
static UI_PATH: &'static str = "/ui";

#[cfg(not(target_os = "redox"))]
static UI_PATH: &'static str = "ui";

#[cfg(not(target_os = "redox"))]
fn wait(status: &mut i32) -> io::Result<usize> {
    extern crate libc;

    use std::io::Error;

    let pid = unsafe { libc::waitpid(0, status as *mut i32, libc::WNOHANG) };
    if pid < 0 {
        Err(io::Error::new(ErrorKind::Other,
                           format!("waitpid failed: {}", Error::last_os_error())))
    }
    Ok(pid as usize)
}

#[cfg(target_os = "redox")]
fn wait(status: &mut i32) -> io::Result<usize> {
    libredox::call::waitpid(0, status, libc::WNOHANG)
        .map_err(|e| io::Error::new(ErrorKind::Other,
                                        format!("Error in waitpid(): {}", e.to_string())))
}

fn load_icon(path: &str) -> Image {
    let icon = Image::from_path(path).unwrap_or(Image::default());
    if icon.width() == icon_size() as u32 && icon.height() == icon_size() as u32 {
        icon
    } else {
        icon.resize(icon_size() as u32, icon_size() as u32, orbimage::ResizeType::Lanczos3).unwrap()
    }
}

fn load_icon_small(path: &str) -> Image {
    let icon = Image::from_path(path).unwrap_or(Image::default());
    if icon.width() == icon_small_size() as u32 && icon.height() == icon_small_size() as u32 {
        icon
    } else {
        icon.resize(icon_small_size() as u32, icon_small_size() as u32, orbimage::ResizeType::Lanczos3).unwrap()
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
            window.rect(0, y, w, icon_small_size() as u32, BAR_HIGHLIGHT_COLOR);
        }

        package.icon_small.draw(window, 0, y);

        font.render(&package.name, font_size() as f32).draw(window, icon_small_size() + 8, y + 8, if i as i32 == selected { TEXT_HIGHLIGHT_COLOR } else { TEXT_COLOR });

        y += icon_small_size();
    }

    window.sync();
}

struct Bar {
    children: Vec<(String, Child)>,
    packages: Vec<Package>,
    start: Image,
    start_packages: Vec<Package>,
    category_packages: BTreeMap<String, Vec<Package>>,
    font: Font,
    width: u32,
    height: u32,
    window: Window,
    selected: i32,
    selected_window: Window,
    time: String,
}

impl Bar {
    fn new(width: u32, height: u32) -> Bar {
        let all_packages = get_packages();

        // Handle packages with categories
        let mut root_packages = Vec::new();
        let mut category_packages = BTreeMap::<String, Vec<Package>>::new();
        for package in all_packages {
            if package.category.is_empty() {
                // Packages without a category go on the bar
                root_packages.push(package);
            } else {
                // Packages with a category are collected
                match category_packages.get_mut(&package.category) {
                    Some(packages) => {
                        packages.push(package);
                    },
                    None => {
                        category_packages.insert(package.category.clone(), vec![package]);
                    },
                }
            }
        }

        let mut start_packages = Vec::new();

        for (category, packages) in category_packages.iter_mut() {
            start_packages.push({
                let mut package = Package::new();
                package.name = category.to_string();
                let icon = format!("{}/icons/mimetypes/inode-directory.png", UI_PATH);
                package.icon = load_icon(&icon);
                package.icon_small = load_icon_small(&icon);
                package.binary = format!("category={}", category);
                package
            });

            packages.insert(0, {
                let mut package = Package::new();
                package.name = "Go back".to_string();
                let icon = format!("{}/icons/mimetypes/inode-directory.png", UI_PATH);
                package.icon = load_icon(&icon);
                package.icon_small = load_icon_small(&icon);
                package.binary = "exit".to_string();
                package
            });
        }

        start_packages.extend_from_slice(&root_packages);

        start_packages.push({
            let mut package = Package::new();
            package.name = "Logout".to_string();
            let icon = format!("{}/icons/actions/system-log-out.png", UI_PATH);
            package.icon = load_icon(&icon);
            package.icon_small = load_icon_small(&icon);
            package.binary = "exit".to_string();
            package
        });

        Bar {
            children: Vec::new(),
            packages: root_packages,
            start: load_icon(&format!("{}/icons/places/start-here.png", UI_PATH)),
            start_packages,
            category_packages,
            font: Font::find(Some("Sans"), None, None).unwrap(),
            width,
            height,
            window: Window::new_flags(
                0, height as i32 - icon_size(), width, icon_size() as u32, "",
                &[WindowFlag::Async, WindowFlag::Borderless, WindowFlag::Transparent]
            ).expect("launcher: failed to open window"),
            selected: -1,
            selected_window: Window::new_flags(
                0, height as i32, width, (font_size() + 8) as u32, "",
                &[WindowFlag::Async, WindowFlag::Borderless, WindowFlag::Transparent],
            ).expect("launcher: failed to open selected window"),
            time: String::new()
        }
    }

    fn update_time(&mut self) {
        let time = libredox::call::clock_gettime(flag::CLOCK_REALTIME).expect("launcher: failed to read time");

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

                self.selected_window.set(Color::rgba(0, 0, 0, 0));

                let text = self.font.render(&package.name, font_size() as f32);
                self.selected_window.rect(x, 0, text.width() + 8, text.height() + 8, BAR_COLOR);
                text.draw(&mut self.selected_window, x + 4, 4, TEXT_HIGHLIGHT_COLOR);

                self.selected_window.sync();
                let sw_y = self.window.y() - self.selected_window.height() as i32 - 4;
                self.selected_window.set_pos(0, sw_y);
            }

            package.icon.draw(&mut self.window, x as i32, y as i32);

            let mut count = 0;
            for (binary, _) in self.children.iter() {
                if binary == &package.binary {
                    count += 1;
                }
            }
            if count > 0 {
                self.window.rect(x as i32 + 4, y as i32,
                                  package.icon.width() - 8, 2,
                                  TEXT_HIGHLIGHT_COLOR);
            }

            x += package.icon.width() as i32;
            i += 1;
        }

        let text = self.font.render(&self.time, (font_size() * 2) as f32);
        x = self.width as i32 - text.width() as i32 - 8;
        y = (icon_size() - text.height() as i32)/2;
        text.draw(&mut self.window, x, y, TEXT_HIGHLIGHT_COLOR);

        self.window.sync();
    }

    fn start_window(&self, category_opt: Option<&String>) -> Option<String> {
        let packages = match category_opt {
            Some(category) => self.category_packages.get(category)?,
            None => &self.start_packages,
        };

        let start_h = packages.len() as u32 * icon_small_size() as u32;
        let mut start_window = Window::new_flags(
            0, self.height as i32 - icon_size() - start_h as i32, chooser_width(), start_h, "Start",
            &[WindowFlag::Borderless, WindowFlag::Transparent]
        ).unwrap();

        let mut selected = -1;
        let mut mouse_y = 0;
        let mut mouse_left = false;
        let mut last_mouse_left = false;
        draw_chooser(&mut start_window, &self.font, packages, selected);
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
                    for (j, _package) in packages.iter().enumerate() {
                        if mouse_y >= y && mouse_y < y + icon_small_size() {
                            now_selected = j as i32;
                        }
                        y += icon_small_size();
                    }

                    if now_selected != selected {
                        selected = now_selected;
                        draw_chooser(&mut start_window, &self.font, packages, selected);
                    }

                    if ! mouse_left && last_mouse_left {
                        let mut y = 0;
                        for package_i in 0..packages.len() {
                            if mouse_y >= y && mouse_y < y + icon_small_size() {
                                return Some(packages[package_i].binary.to_string());
                            }
                            y += icon_small_size();
                        }
                    }

                    last_mouse_left = mouse_left;
                }
            }
        }
        None
    }

    fn spawn(&mut self, binary: String) {
        match Command::new(&binary).spawn() {
            Ok(child) => {
                self.children.push((binary, child));
                //TODO: should redraw be done here?
                self.draw();
            },
            Err(err) => error!("failed to spawn {}: {}", binary, err)
        }
    }
}

fn bar_main(width: u32, height: u32) -> io::Result<()> {
    let mut bar = Bar::new(width, height);

    match Command::new("background").spawn() {
        Ok(child) => bar.children.push(("background".to_string(), child)),
        Err(err) => error!("failed to launch background: {}", err)
    }

    user_data! {
        enum Event {
            Time,
            Window,
        }
    }
    let event_queue = EventQueue::<Event>::new().expect("launcher: failed to create event queue");

    let mut time_file = File::open(&format!("time:{}", flag::CLOCK_MONOTONIC))?;

    event_queue.subscribe(time_file.as_raw_fd() as usize, Event::Time, event::EventFlags::READ)
        .expect("launcher: failed to subscribe to timer");
    event_queue.subscribe(bar.window.as_raw_fd() as usize, Event::Window, event::EventFlags::READ)
        .expect("launcher: failed to subscribe to timer");

    let mut mouse_x = -1;
    let mut mouse_y = -1;
    let mut mouse_left = false;
    let mut last_mouse_left = false;

    let all_events = core::array::IntoIter::new([Event::Time, Event::Window]);

    'events: for event in all_events.chain(event_queue.map(|e| e.expect("launcher: failed to get next event").user_data)) {
        match event {
            Event::Time => {
                let mut time_buf = [0_u8; core::mem::size_of::<TimeSpec>()];
                if time_file.read(&mut time_buf)? < mem::size_of::<TimeSpec>() {
                    continue;
                }

                let mut i = 0;
                while i < bar.children.len() {
                    let remove = match bar.children[i].1.try_wait() {
                        Ok(None) => false,
                        Ok(Some(status)) => {
                            info!("{} ({}) exited with {}", bar.children[i].0, bar.children[i].1.id(), status);
                            true
                        },
                        Err(err) => {
                            error!("failed to wait for {} ({}): {}", bar.children[i].0, bar.children[i].1.id(), err);
                            true
                        }
                    };
                    if remove {
                        bar.children.remove(i);
                    } else {
                        i += 1;
                    }
                }

                loop {
                    let mut status = 0;
                    let pid = wait(&mut status)?;
                    if pid == 0 {
                        break;
                    }
                }

                bar.update_time();
                bar.draw();

                match libredox::data::timespec_from_mut_bytes(&mut time_buf) {
                    time => {
                        time.tv_sec += 1;
                        time.tv_nsec = 0;
                    }
                }
                time_file.write(&time_buf)?;
            }
            Event::Window => {
                for event in bar.window.events() {
                    //TODO: remove hack for super event
                    if event.code >= 0x1000_0000 {
                        let mut super_event = event;
                        super_event.code -= 0x1000_0000;

                        //TODO: configure super keybindings
                        let event_option = super_event.to_option();
                        debug!("launcher: super {:?}", event_option);
                        match event_option {
                            EventOption::Key(key_event) => match key_event.scancode {
                                orbclient::K_B => if key_event.pressed {
                                    bar.spawn("netsurf-fb".to_string());
                                },
                                orbclient::K_F => if key_event.pressed {
                                    bar.spawn("file_manager".to_string());
                                },
                                orbclient::K_T => if key_event.pressed {
                                    bar.spawn("orbterm".to_string());
                                },
                                _ => (),
                            }
                            _ => (),
                        }

                        continue;
                    }

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
                            bar.window.set_pos(0, screen_event.height as i32 - icon_size());
                            bar.window.set_size(screen_event.width, icon_size() as u32);
                            bar.selected = -2; // Force bar redraw
                            bar.selected_window.set_pos(0, screen_event.height as i32);
                            bar.selected_window.set_size(screen_event.width, (font_size() + 8) as u32);
                            true
                        },
                        EventOption::Hover(hover_event) => if hover_event.entered {
                            false
                        } else {
                            mouse_x = -1;
                            mouse_y = -1;
                            true
                        },
                        EventOption::Quit(_) => break 'events,
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
                            let sw_y = bar.height as i32;
                            bar.selected_window.set_pos(0, sw_y);
                            bar.draw();
                        }

                        if ! mouse_left && last_mouse_left {
                            let mut i = 0;

                            if i == bar.selected {
                                let mut category_opt = None;
                                while let Some(binary) = bar.start_window(category_opt.as_ref()) {
                                    if binary.starts_with("category=") {
                                        let category = &binary[9..];
                                        category_opt = Some(category.to_string());
                                    } else if binary == "exit" {
                                        if category_opt.is_some() {
                                            category_opt = None;
                                        } else {
                                            break 'events;
                                        }
                                    } else {
                                        bar.spawn(binary);
                                        break;
                                    }
                                }
                            }
                            i += 1;

                            for package_i in 0..bar.packages.len() {
                                if i == bar.selected {
                                    let binary = bar.packages[package_i].binary.clone();
                                    bar.spawn(binary);
                                }
                                i += 1;
                            }
                        }

                        last_mouse_left = mouse_left;
                    }
                }
            }
        }

    }

    debug!("Launcher exiting, killing {} children", bar.children.len());
    for (binary, child) in bar.children.iter_mut() {
        let pid = child.id();
        match child.kill() {
            Ok(()) => debug!("Successfully killed child: {}", pid),
            Err(err) => error!("failed to kill {} ({}): {}", binary, pid, err),
        }
        match child.wait() {
            Ok(status) => info!("{} ({}) exited with {}", binary, pid, status),
            Err(err) => error!("failed to wait for {} ({}): {}", binary, pid, err),
        }
    }

    // kill any descendents of one of the children killed above that are still running
    debug!("Launcher exiting, reaping all zombie processes");
    let mut status = 0;
    while wait(&mut status).is_ok() {};

    Ok(())
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
            let mut window = Window::new(-1, -1, chooser_width(), packages.len() as u32 * icon_small_size() as u32, path).expect("launcher: failed to open window");
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
                            if mouse_y >= y && mouse_y < y + icon_size() {
                                now_selected = i as i32;
                            }
                            y += icon_small_size();
                        }

                        if now_selected != selected {
                            selected = now_selected;
                            draw_chooser(&mut window, &font, &packages, selected);
                        }

                        if ! mouse_left && last_mouse_left {
                            let mut y = 0;
                            for package in packages.iter() {
                                if mouse_y >= y && mouse_y < y + icon_small_size() {
                                    if let Err(err) = Command::new(&package.binary).arg(path).spawn() {
                                        error!("failed to launch {}: {}", package.binary, err);
                                    }
                                    break 'choosing;
                                }
                                y += icon_small_size();
                            }
                        }

                        last_mouse_left = mouse_left;
                    }
                }
            }
        } else if let Some(package) = packages.get(0) {
            if let Err(err) = Command::new(&package.binary).arg(&path).spawn() {
                error!("failed to launch '{}': {}", package.binary, err);
            }
        } else {
            error!("no application found for '{}'", path);
        }
    }
}

fn start_logging() {
    if let Err(e) = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("launcher".into())
        .enable() {
        eprintln!("Launcher could not start logging: {}", e);
    }
}

fn main() -> Result<(), String>{
    start_logging();

    let (width, height) = orbclient::get_display_size()?;
    SCALE.store((height as isize / 1600) + 1, Ordering::Relaxed);
    let paths = env::args();
    if paths.len() > 1 {
        chooser_main(paths);
    } else {
        bar_main(width, height).map_err(|e| e.to_string())?;
    }

    Ok(())
}
