#![deny(warnings)]
#![feature(inclusive_range_syntax)]

extern crate orbclient;
extern crate orbimage;
extern crate orbfont;

use std::{cmp, env};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::string::{String, ToString};
use std::vec::Vec;

use orbclient::{event, Color, EventOption, MouseEvent, Window};
use orbimage::Image;
use orbfont::Font;

#[cfg(target_os = "redox")]
static UI_PATH: &'static str = "/ui";

#[cfg(not(target_os = "redox"))]
static UI_PATH: &'static str = "ui";

#[cfg(target_os = "redox")]
static LAUNCH_COMMAND: &'static str = "/ui/bin/launcher";

#[cfg(not(target_os = "redox"))]
static LAUNCH_COMMAND: &'static str = "xdg-open";

struct FileInfo {
    name: String,
    size: u64,
    size_str: String,
    is_dir: bool,
}

impl FileInfo {
    fn new(name: String, is_dir: bool) -> FileInfo {
        let (size, size_str) = {
            if is_dir {
                FileManager::get_num_entries(&name)
            } else {
                match fs::metadata(&name) {
                    Ok(metadata) => {
                        let size = metadata.len();
                        if size >= 1_000_000_000 {
                            (size, format!("{:.1} GB", (size as u64) / 1_000_000_000))
                        } else if size >= 1_000_000 {
                            (size, format!("{:.1} MB", (size as u64) / 1_000_000))
                        } else if size >= 1_000 {
                            (size, format!("{:.1} KB", (size as u64) / 1_000))
                        } else {
                            (size, format!("{:.1} bytes", size))
                        }
                    }
                    Err(err) => (0, format!("Failed to open: {}", err)),
                }
            }
        };
        FileInfo {
            name: name,
            size: size,
            size_str: size_str,
            is_dir: is_dir,
        }
    }
}

struct FileType {
    description: &'static str,
    icon: &'static str,
}


impl FileType {
    fn new(desc: &'static str, icon: &'static str) -> FileType {
        FileType {
            description: desc,
            icon: icon,
        }
    }
}

struct FileTypesInfo {
    file_types: BTreeMap<&'static str, FileType>,
    images: BTreeMap<&'static str, Image>,
}

impl FileTypesInfo {
    pub fn new() -> FileTypesInfo {
        let mut file_types = BTreeMap::<&'static str, FileType>::new();
        file_types.insert("/", FileType::new("Folder", "inode-directory"));
        file_types.insert("wav", FileType::new("WAV audio", "audio-x-wav"));
        file_types.insert("bin",
                          FileType::new("Executable", "application-x-executable"));
        file_types.insert("bmp", FileType::new("Bitmap Image", "image-x-generic"));
        file_types.insert("jpg", FileType::new("JPG Image", "image-x-generic"));
        file_types.insert("jpeg", FileType::new("JPG Image", "image-x-generic"));
        file_types.insert("png", FileType::new("PNG Image", "image-x-generic"));
        file_types.insert("rs", FileType::new("Rust source code", "text-x-makefile"));
        file_types.insert("crate",
                          FileType::new("Rust crate", "application-x-archive"));
        file_types.insert("rlib",
                          FileType::new("Static Rust library", "application-x-object"));
        file_types.insert("asm", FileType::new("Assembly source", "text-x-makefile"));
        file_types.insert("list",
                          FileType::new("Disassembly source", "text-x-makefile"));
        file_types.insert("c", FileType::new("C source code", "text-x-csrc"));
        file_types.insert("cpp", FileType::new("C++ source code", "text-x-c++src"));
        file_types.insert("h", FileType::new("C header", "text-x-chdr"));
        file_types.insert("ion", FileType::new("Ion script", "text-x-script"));
        file_types.insert("rc", FileType::new("Init script", "text-x-script"));
        file_types.insert("sh", FileType::new("Shell script", "text-x-script"));
        file_types.insert("lua", FileType::new("Lua script", "text-x-script"));
        file_types.insert("conf", FileType::new("Config file", "text-x-generic"));
        file_types.insert("txt", FileType::new("Plain text file", "text-x-generic"));
        file_types.insert("md", FileType::new("Markdown file", "text-x-generic"));
        file_types.insert("toml", FileType::new("TOML file", "text-x-generic"));
        file_types.insert("json", FileType::new("JSON file", "text-x-generic"));
        file_types.insert("REDOX", FileType::new("Redox package", "text-x-generic"));
        file_types.insert("", FileType::new("Unknown file", "unknown"));
        FileTypesInfo { file_types: file_types, images: BTreeMap::new() }
    }

    pub fn description_for(&self, file_name: &str) -> String {
        if file_name.ends_with('/') {
            self.file_types["/"].description.to_owned()
        } else {
            let pos = file_name.rfind('.').unwrap_or(0) + 1;
            let ext = &file_name[pos..];
            if self.file_types.contains_key(ext) {
                self.file_types[ext].description.to_string()
            } else {
                self.file_types[""].description.to_string()
            }
        }
    }

    pub fn icon_for(&mut self, file_name: &str) -> &Image {
        let icon = if file_name.ends_with('/') {
            &self.file_types["/"].icon
        } else {
            let pos = file_name.rfind('.').unwrap_or(0) + 1;
            let ext = &file_name[pos..];
            if self.file_types.contains_key(ext) {
                &self.file_types[ext].icon
            } else {
                &self.file_types[""].icon
            }
        };

        if ! self.images.contains_key(icon) {
            self.images.insert(icon, load_icon(icon));
        }
        &self.images[icon]
    }
}

enum FileManagerCommand {
    ChangeDir(String),
    Execute(String),
    Redraw,
    Quit,
}

#[derive(PartialEq)]
enum SortPredicate {
    Name,
    Size,
    Type,
}

#[derive(PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    fn invert(&mut self) {
        match *self {
            SortDirection::Asc => *self = SortDirection::Desc,
            SortDirection::Desc => *self = SortDirection::Asc,
        }
    }
}

pub struct FileManager {
    file_types_info: FileTypesInfo,
    files: Vec<FileInfo>,
    selected: isize,
    column: [i32; 3], // The x-coordinates of the "size" and "type" columns
    sort_predicate: SortPredicate,
    sort_direction: SortDirection,
    last_mouse_event: MouseEvent,
    window: Window,
    font: Font,
}

fn load_icon(path: &str) -> Image {
    println!("Load {}", path);
    match Image::from_path(&format!("{}/icons/mimetypes/{}.png", UI_PATH, path)) {
        Ok(icon) => icon,
        Err(err) => {
            println!("Failed to load icon {}: {}", path, err);
            Image::new(32, 32)
        }
    }
}

impl FileManager {
    pub fn new() -> Self {
        FileManager {
            file_types_info: FileTypesInfo::new(),
            files: Vec::new(),
            selected: -1,
            column: [0, 0, 0],
            sort_predicate: SortPredicate::Name,
            sort_direction: SortDirection::Asc,
            last_mouse_event: MouseEvent {
                x: 0,
                y: 0,
                left_button: false,
                middle_button: false,
                right_button: false,
            },
            window: Window::new(-1, -1, 0, 0, "").unwrap(),
            font: Font::find(None, None, None).unwrap()
        }
    }

    fn draw_content(&mut self) {
        self.window.set(Color::rgb(255, 255, 255));
        self.draw_header_row();
        self.draw_file_list();
        self.window.sync();
    }

    fn draw_sort_direction_arrow(&mut self, x: i32, y: i32) {
        let color = Color::rgb(140, 140, 140);
        let tip_y = match self.sort_direction {
            SortDirection::Asc => 2,
            SortDirection::Desc => -2,
        };
        for dy in -1...0 {
            for dx in -2...2 { self.window.pixel(x + dx, y + dy - tip_y, color); }
            for dx in -1...1 { self.window.pixel(x + dx, y + dy, color); }
            self.window.pixel(x, y + dy + tip_y, color);
        }
    }

    fn draw_header_row(&mut self) {
        // TODO: Remove duplication between this function and draw_file_list()
        let row = 0;

        self.font.render("Name", 16.0).draw(&mut self.window, self.column[0], 32 * row as i32 + 8, Color::rgb(0, 0, 0));
        self.font.render("Size", 16.0).draw(&mut self.window, self.column[1], 32 * row as i32 + 8, Color::rgb(0, 0, 0));
        self.font.render("Type", 16.0).draw(&mut self.window, self.column[2], 32 * row as i32 + 8, Color::rgb(0, 0, 0));

        let column = self.column;
        match self.sort_predicate {
            SortPredicate::Name => self.draw_sort_direction_arrow(column[0] + 80, 16),
            SortPredicate::Size => self.draw_sort_direction_arrow(column[1] + 80, 16),
            SortPredicate::Type => self.draw_sort_direction_arrow(column[2] + 80, 16),
        }
    }

    fn draw_file_list(&mut self) {
        let mut i = 0;
        let mut row = 1; // Start at 1 because the header row is 0
        for file in self.files.iter() {
            if i == self.selected {
                let width = self.window.width();
                self.window.rect(0,
                                 32 * row as i32,
                                 width,
                                 32,
                                 Color::rgba(224, 224, 224, 255));
            }

            {
                let icon = self.file_types_info.icon_for(&file.name);
                icon.draw(&mut self.window, 0, 32 * row as i32);
            }

            self.font.render(&file.name, 16.0).draw(&mut self.window, self.column[0], 32 * row as i32 + 8, Color::rgb(0, 0, 0));
            self.font.render(&file.size_str, 16.0).draw(&mut self.window, self.column[1], 32 * row as i32 + 8, Color::rgb(0, 0, 0));

            let description = self.file_types_info.description_for(&file.name);
            self.font.render(&description, 16.0).draw(&mut self.window, self.column[2], 32 * row as i32 + 8, Color::rgb(0, 0, 0));

            row += 1;
            i += 1;
        }
    }

    fn get_parent_directory() -> Option<String> {
        match fs::canonicalize("../") {
            Ok(path) => return Some(path.into_os_string().into_string().unwrap_or("/".to_string())),
            Err(err) => println!("failed to get path: {}", err)
        }

        None
    }

    fn get_num_entries(path: &str) -> (u64, String) {
        let count = match fs::read_dir(path) {
            Ok(entry_readdir) => entry_readdir.count(),
            Err(_) => 0,
        };
        if count == 1 {
            (count as u64, "1 entry".to_string())
        } else {
            (count as u64, format!("{} entries", count))
        }
    }

    fn set_path(&mut self, path: &str) {
        let mut width = [48; 3];
        let mut height = 0;

        if let Err(err) = env::set_current_dir(path) {
            println!("failed to set dir {}: {}", path, err);
        }

        match fs::read_dir(path) {
            Ok(readdir) => {
                self.files.clear();

                // check to see if parent directory exists
                if let Some(_) = FileManager::get_parent_directory() {
                    self.files.push(FileInfo::new("../".to_string(), true));
                }

                for entry_result in readdir {
                    match entry_result {
                        Ok(entry) => {
                            let directory = match entry.file_type() {
                                Ok(file_type) => file_type.is_dir(),
                                Err(err) => {
                                    println!("Failed to read file type: {}", err);
                                    false
                                }
                            };

                            let entry_path = match entry.file_name().to_str() {
                                Some(path_str) => if directory {
                                    path_str.to_string() + "/"
                                } else {
                                    path_str.to_string()
                                },
                                None => {
                                    println!("Failed to read file name");
                                    String::new()
                                }
                            };

                            let file_info = FileInfo::new(entry_path, directory);

                            // Unwrapping the last file size will not panic since it has
                            // been at least pushed once in the vector
                            let description = self.file_types_info.description_for(&file_info.name);
                            width[0] = cmp::max(width[0], 40 + (file_info.name.len() * 8) + 16);
                            width[1] = cmp::max(width[1], 8 + (file_info.size_str.len() * 8) + 16);
                            width[2] = cmp::max(width[2], 16 + (description.len() * 8) + 16);

                            self.files.push(file_info);
                        },
                        Err(err) => println!("failed to read dir entry: {}", err)
                    }
                }

                self.sort_files();

                self.column = [40, width[0] as i32 + 8, width[0] as i32 + width[1] as i32 + 8];

                height = cmp::max(height, (self.files.len() + 1) * 32) // +1 for the header row
            },
            Err(err) => println!("failed to readdir {}: {}", path, err)
        }

        // TODO: HACK ALERT - should use resize whenver that gets added
        self.window.sync_path();

        let x = self.window.x();
        let y = self.window.y();
        let w = width.iter().sum::<usize>() as u32;
        let h = height as u32;

        self.window = Window::new(x, y, w, h, &path).unwrap();

        self.draw_content();
    }

    fn sort_files(&mut self) {
        match self.sort_predicate {
            SortPredicate::Name => self.files.sort_by(|a, b| a.name.cmp(&b.name)),
            SortPredicate::Size => {
                self.files.sort_by(|a, b|
                    if a.is_dir != b.is_dir {
                        b.is_dir.cmp(&a.is_dir) // Sort directories first
                    } else {
                        a.size.cmp(&b.size)
                    })
            },
            SortPredicate::Type => {
                let file_types_info = &self.file_types_info;
                self.files.sort_by_key(|file| file_types_info.description_for(&file.name).to_lowercase())
            },
        }
        if self.sort_direction == SortDirection::Desc {
            self.files.reverse();
        }
    }

    fn event_loop(&mut self) -> Vec<FileManagerCommand> {
        let mut redraw = false;
        let mut commands = Vec::new();
        for event in self.window.events() {
            match event.to_option() {
                EventOption::Key(key_event) => {
                    if key_event.pressed {
                        match key_event.scancode {
                            event::K_ESC => commands.push(FileManagerCommand::Quit),
                            event::K_HOME => self.selected = 0,
                            event::K_UP => {
                                if self.selected > 0 {
                                    self.selected -= 1;
                                    redraw = true;
                                }
                            },
                            event::K_END => self.selected = self.files.len() as isize - 1,
                            event::K_DOWN => {
                                if self.selected < self.files.len() as isize - 1 {
                                    self.selected += 1;
                                    redraw = true;
                                }
                            },
                            _ => {
                                match key_event.character {
                                    '\0' => (),
                                    '\n' => {
                                        if self.selected >= 0 &&
                                           self.selected < self.files.len() as isize {
                                            match self.files.get(self.selected as usize) {
                                                Some(file) => {
                                                    if file.name.ends_with('/') {
                                                        commands.push(FileManagerCommand::ChangeDir(file.name.clone()));
                                                    } else {
                                                        commands.push(FileManagerCommand::Execute(file.name.clone()));
                                                    }
                                                }
                                                None => (),
                                            }
                                        }
                                    }
                                    _ => {
                                        let mut i = 0;
                                        for file in self.files.iter() {
                                            if file.name.starts_with(key_event.character) {
                                                self.selected = i;
                                                break;
                                            }
                                            i += 1;
                                        }
                                    }
                                }
                            }
                        }
                        if redraw {
                            commands.push(FileManagerCommand::Redraw);
                        }
                    }
                }
                EventOption::Mouse(mouse_event) => {
                    redraw = false;
                    let mut i = 0;
                    let mut row = 0;
                    for file in self.files.iter() {
                        let mut col = 0;
                        for c in file.name.chars() {
                            if mouse_event.y >= 32 * (row as i32 + 1) && // +1 for the header row
                               mouse_event.y < 32 * (row as i32 + 2) {
                                if i != self.selected {
                                    self.selected = i;
                                    redraw = true;
                                }
                            }

                            if c == '\n' {
                                col = 0;
                                row += 1;
                            } else if c == '\t' {
                                col += 8 - col % 8;
                            } else {
                                if col < self.window.width() / 8 &&
                                   row < self.window.height() / 32 {
                                    col += 1;
                                }
                            }
                            if col >= self.window.width() / 8 {
                                col = 0;
                                row += 1;
                            }
                        }
                        row += 1;
                        i += 1;
                    }

                    if ! mouse_event.left_button && self.last_mouse_event.left_button {
                        if mouse_event.y < 32 { // Header row clicked
                            if mouse_event.x < self.column[1] as i32 {
                                if self.sort_predicate != SortPredicate::Name {
                                    self.sort_predicate = SortPredicate::Name;
                                } else {
                                    self.sort_direction.invert();
                                }
                            } else if mouse_event.x < self.column[2] as i32 {
                                if self.sort_predicate != SortPredicate::Size {
                                    self.sort_predicate = SortPredicate::Size;
                                } else {
                                    self.sort_direction.invert();
                                }
                            } else {
                                if self.sort_predicate != SortPredicate::Type {
                                    self.sort_predicate = SortPredicate::Type;
                                } else {
                                    self.sort_direction.invert();
                                }
                            }
                            self.sort_files();
                            redraw = true;
                        } else if self.last_mouse_event.x == mouse_event.x &&
                                  self.last_mouse_event.y == mouse_event.y {
                            if self.selected >= 0 && self.selected < self.files.len() as isize {
                                if let Some(file) = self.files.get(self.selected as usize) {
                                    if file.name.ends_with('/') {
                                        commands.push(FileManagerCommand::ChangeDir(file.name.clone()));
                                    } else {
                                        commands.push(FileManagerCommand::Execute(file.name.clone()));
                                    }
                                }
                            }
                        }
                    }
                    self.last_mouse_event = mouse_event;

                    if redraw {
                        commands.push(FileManagerCommand::Redraw);
                    }
                }
                EventOption::Quit(_) => commands.push(FileManagerCommand::Quit),
                _ => (),
            }
        }
        commands
    }

    fn main(&mut self, path: &str) {
        let mut current_path = path.to_string();
        if ! current_path.ends_with('/') {
            current_path.push('/');
        }
        self.set_path(path);
        self.draw_content();
        'events: loop {
            let mut redraw = false;
            for event in self.event_loop() {
                match event {
                    FileManagerCommand::ChangeDir(dir) => {
                        if dir == "../" {
                            if let Some(parent_dir) = FileManager::get_parent_directory() {
                                current_path = parent_dir;
                                if ! current_path.ends_with('/') {
                                    current_path.push('/');
                                }
                            }
                        } else {
                            if ! current_path.ends_with('/') {
                                current_path.push('/');
                            }
                            current_path.push_str(&dir);
                        }
                        self.set_path(&current_path);
                    }
                    FileManagerCommand::Execute(cmd) => {
                        Command::new(LAUNCH_COMMAND).arg(&(current_path.clone() + &cmd)).spawn().unwrap();
                    },
                    FileManagerCommand::Redraw => redraw = true,
                    FileManagerCommand::Quit => break 'events,
                };
            }
            if redraw {
                self.draw_content();
            }
        }
    }
}

fn main() {
    match env::args().nth(1) {
        Some(ref arg) => FileManager::new().main(arg),
        None => if let Some(home) = env::home_dir() {
            FileManager::new().main(home.into_os_string().to_str().unwrap_or("."))
        } else {
            FileManager::new().main(".")
        }
    }
}
