#![deny(warnings)]
#![feature(inclusive_range_syntax)]

extern crate orbclient;
extern crate orbimage;
extern crate orbfont;

use std::{cmp, env, fs};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::{String, ToString};
use std::vec::Vec;

use orbclient::{event, Color, EventOption, MouseEvent, Renderer, Window};
use orbimage::Image;
use orbfont::Font;

const ICON_SIZE: i32 = 32;

#[cfg(target_os = "redox")]
static UI_PATH: &'static str = "/ui/icons";

#[cfg(not(target_os = "redox"))]
static UI_PATH: &'static str = "ui/icons";

#[cfg(target_os = "redox")]
static LAUNCH_COMMAND: &'static str = "/ui/bin/launcher";

#[cfg(not(target_os = "redox"))]
static LAUNCH_COMMAND: &'static str = "xdg-open";

struct FileInfo {
    name: String,
    full_path: String,
    size: u64,
    size_str: String,
    is_dir: bool,
}

impl FileInfo {
    fn new(name: String, full_path: String, is_dir: bool) -> FileInfo {
        let (size, size_str) = {
            if is_dir {
                FileManager::get_num_entries(&full_path)
            } else {
                match fs::metadata(&full_path) {
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
            full_path: full_path,
            size: size,
            size_str: size_str,
            is_dir: is_dir,
        }
    }
}

struct FileType {
    description: &'static str,
    icon: PathBuf
}

impl FileType {
    fn new(desc: &'static str, icon: &'static str) -> FileType {
        for folder in ["mimetypes", "places"].iter() {
            let mut path = fs::canonicalize(UI_PATH).unwrap();
            path.push(folder);
            path.push(format!("{}.png", icon));
            if path.is_file() {
                return FileType {
                    description: desc,
                    icon: path,
                };
            } else {
                println!("{} not found in {}", icon, folder);
            }
        }

        println!("{} not found", icon);
        let mut path = fs::canonicalize(UI_PATH).unwrap();
        path.push("mimetypes/unknown.png");
        FileType {
            description: desc,
            icon: path,
        }
    }
}

struct FileTypesInfo {
    file_types: BTreeMap<&'static str, FileType>,
    images: BTreeMap<PathBuf, Image>,
}

impl FileTypesInfo {
    pub fn new() -> FileTypesInfo {
        let mut file_types = BTreeMap::<&'static str, FileType>::new();
        file_types.insert("/", FileType::new("Folder", "inode-directory"));

        // Archives
        file_types.insert("tar", FileType::new("TAR Archive", "package-x-generic"));

        // Audio formats
        file_types.insert("wav", FileType::new("WAV audio", "audio-x-generic"));

        // Font formats
        file_types.insert("ttf", FileType::new("TTF Font", "application-x-font-ttf"));

        // Image formats
        file_types.insert("bmp", FileType::new("Bitmap Image", "image-x-generic"));
        file_types.insert("jpg", FileType::new("JPG Image", "image-x-generic"));
        file_types.insert("jpeg", FileType::new("JPG Image", "image-x-generic"));
        file_types.insert("png", FileType::new("PNG Image", "image-x-generic"));

        // Text formats
        file_types.insert("txt", FileType::new("Text file", "text-plain"));

        // Markdown formats
        file_types.insert("md", FileType::new("Markdown file", "text-plain"));

        // Configuration formats
        file_types.insert("conf", FileType::new("Config file", "text-plain"));
        file_types.insert("json", FileType::new("JSON file", "text-plain"));
        file_types.insert("toml", FileType::new("TOML file", "text-plain"));

        // C programming language formats
        file_types.insert("c", FileType::new("C source", "text-x-c"));
        file_types.insert("cpp", FileType::new("C++ source", "text-x-c"));
        file_types.insert("h", FileType::new("C header", "text-x-c"));

        // Programming language formats
        file_types.insert("asm", FileType::new("Assembly source", "text-x-script"));
        file_types.insert("ion", FileType::new("Ion script", "text-x-script"));
        file_types.insert("lua", FileType::new("Lua script", "text-x-script"));
        file_types.insert("rc", FileType::new("Init script", "text-x-script"));
        file_types.insert("rs", FileType::new("Rust source", "text-x-script"));
        file_types.insert("sh", FileType::new("Shell script", "text-x-script"));

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
            self.images.insert(icon.clone(), load_icon(icon));
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

struct Column {
    name: &'static str,
    x: i32,
    width: i32,
    sort_predicate: SortPredicate,
}

pub struct FileManager {
    file_types_info: FileTypesInfo,
    files: Vec<FileInfo>,
    selected: isize,
    columns: [Column; 3],
    sort_predicate: SortPredicate,
    sort_direction: SortDirection,
    last_mouse_event: MouseEvent,
    window: Window,
    font: Font,
}

fn load_icon(path: &Path) -> Image {
    match Image::from_path(path) {
        Ok(icon) => if icon.width() == ICON_SIZE as u32 && icon.height() == ICON_SIZE as u32 {
            icon
        } else {
            icon.resize(ICON_SIZE as u32, ICON_SIZE as u32, orbimage::ResizeType::Lanczos3).unwrap()
        },
        Err(err) => {
            println!("Failed to load icon {}: {}", path.display(), err);
            Image::from_color(ICON_SIZE as u32, ICON_SIZE as u32, Color::rgba(0, 0, 0, 0))
        }
    }
}

impl FileManager {
    pub fn new() -> Self {
        FileManager {
            file_types_info: FileTypesInfo::new(),
            files: Vec::new(),
            selected: -1,
            columns: [
                Column {
                    name: "Name",
                    x: 0,
                    width: 0,
                    sort_predicate: SortPredicate::Name,
                },
                Column {
                    name: "Size",
                    x: 0,
                    width: 0,
                    sort_predicate: SortPredicate::Size,
                },
                Column {
                    name: "Type",
                    x: 0,
                    width: 0,
                    sort_predicate: SortPredicate::Type,
                },
            ],
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

    fn draw_header_row(&mut self) {
        for column in self.columns.iter() {
            let text_y = 8;

            self.font.render(column.name, 16.0).draw(&mut self.window, column.x, text_y, Color::rgb(0, 0, 0));
            if column.sort_predicate == self.sort_predicate {
                let arrow = match self.sort_direction {
                    SortDirection::Asc => self.font.render("↓", 16.0),
                    SortDirection::Desc => self.font.render("↑", 16.0),
                };
                let arrow_x = column.x + column.width - arrow.width() as i32 - 4;
                arrow.draw(&mut self.window, arrow_x, text_y, Color::rgb(140, 140, 140));
            }
        }
    }

    fn draw_file_list(&mut self) {
        for (i, file) in self.files.iter().enumerate() {
            let y = ICON_SIZE * i as i32 + 32; // Plus 32 because the header row is 32 pixels

            let text_color = if i as isize == self.selected {
                let width = self.window.width();
                self.window.rect(0, y, width, ICON_SIZE as u32, Color::rgb(0x52, 0x94, 0xE2));
                Color::rgb(255, 255, 255)
            } else {
                Color::rgb(0, 0, 0)
            };

            {
                let icon = self.file_types_info.icon_for(&file.name);
                icon.draw(&mut self.window, 4, y);
            }

            self.font.render(&file.name, 16.0).draw(&mut self.window, self.columns[0].x, y + 8, text_color);
            self.font.render(&file.size_str, 16.0).draw(&mut self.window, self.columns[1].x, y + 8, text_color);

            let description = self.file_types_info.description_for(&file.name);
            self.font.render(&description, 16.0).draw(&mut self.window, self.columns[2].x, y + 8, text_color);
        }
    }

    fn get_parent_directory(path: &str) -> Option<String> {
        match fs::canonicalize(path.to_owned() + "../") {
            Ok(parent) => {
                let mut parent = parent.into_os_string().into_string().unwrap_or("/".to_string());
                if ! parent.ends_with('/') {
                    parent.push('/');
                }

                if parent == path {
                    return None
                } else {
                    return Some(parent);
                }
            },
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

    fn push_file(&mut self, file_info: FileInfo) {
        let description = self.file_types_info.description_for(&file_info.name);
        self.columns[0].width = cmp::max(self.columns[0].width, (file_info.name.len() * 8) as i32 + 16);
        self.columns[1].width = cmp::max(self.columns[1].width, (file_info.size_str.len() * 8) as i32 + 16);
        self.columns[2].width = cmp::max(self.columns[2].width, (description.len() * 8) as i32 + 16);

        self.files.push(file_info);
    }

    fn set_path(&mut self, path: &str) {
        for column in self.columns.iter_mut() {
            column.width = (column.name.len() * 8) as i32 + 16;
        }

        self.files.clear();

        // check to see if parent directory exists
        if let Some(parent) = FileManager::get_parent_directory(path) {
            self.push_file(FileInfo::new("../".to_string(), parent, true));
        }

        match fs::read_dir(path) {
            Ok(readdir) => {
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

                            let full_path = path.to_owned() + entry_path.clone().as_str();
                            self.push_file(FileInfo::new(entry_path, full_path, directory));
                        },
                        Err(err) => println!("failed to read dir entry: {}", err)
                    }
                }

            },
            Err(err) => {
                println!("failed to readdir {}: {}", path, err);
            },
        }

        self.sort_files();

        self.columns[0].x = ICON_SIZE + 8;
        self.columns[1].x = self.columns[0].x + self.columns[0].width;
        self.columns[2].x = self.columns[1].x + self.columns[1].width;

        // TODO: HACK ALERT - should use resize whenver that gets added
        self.window.sync_path();

        let x = self.window.x();
        let y = self.window.y();
        let w = (self.columns[2].x + self.columns[2].width) as u32;
        let h = (self.files.len() * ICON_SIZE as usize) as u32 + 32; // +32 for the header row

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
                            event::K_HOME => {
                                self.selected = 0;
                                redraw = true;
                            },
                            event::K_UP => {
                                if self.selected > 0 {
                                    self.selected -= 1;
                                    redraw = true;
                                }
                            },
                            event::K_END => {
                                self.selected = self.files.len() as isize - 1;
                                redraw = true;
                            },
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
                                                    if file.full_path.ends_with('/') {
                                                        commands.push(FileManagerCommand::ChangeDir(file.full_path.clone()));
                                                    } else {
                                                        commands.push(FileManagerCommand::Execute(file.full_path.clone()));
                                                    }
                                                }
                                                None => (),
                                            }
                                        }
                                    }
                                    _ => {
                                        // The index of the first matching file
                                        let mut result_first: Option<isize> = None;
                                        // The index of the next matching file relative to the current selection
                                        let mut result_next: Option<isize> = None;

                                        for (i, file) in self.files.iter().enumerate() {
                                            if file.name.to_lowercase().starts_with(key_event.character) {
                                                if result_first.is_none() {
                                                    result_first = Some(i as isize);
                                                }

                                                if i as isize > self.selected {
                                                    result_next = Some(i as isize);
                                                    break;
                                                }
                                            }
                                        }

                                        redraw = true;
                                        self.selected = result_next.or(result_first).unwrap_or(-1);
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

                    for (row, _) in self.files.iter().enumerate() {
                        if mouse_event.y >= ICON_SIZE * (row as i32 + 1) && // +1 for the header row
                           mouse_event.y < ICON_SIZE * (row as i32 + 2) {
                            if row as isize != self.selected {
                                self.selected = row as isize;
                                redraw = true;
                            }
                        }
                    }

                    if ! mouse_event.left_button && self.last_mouse_event.left_button {
                        if mouse_event.y < ICON_SIZE { // Header row clicked
                            if mouse_event.x < self.columns[1].x as i32 {
                                if self.sort_predicate != SortPredicate::Name {
                                    self.sort_predicate = SortPredicate::Name;
                                } else {
                                    self.sort_direction.invert();
                                }
                            } else if mouse_event.x < self.columns[2].x as i32 {
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
                                    if file.full_path.ends_with('/') {
                                        commands.push(FileManagerCommand::ChangeDir(file.full_path.clone()));
                                    } else {
                                        commands.push(FileManagerCommand::Execute(file.full_path.clone()));
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
        // Filter out invalid paths
        let mut path = match fs::canonicalize(path.to_owned()) {
            Ok(p) => p.into_os_string().into_string().unwrap_or("file:/".to_owned()),
            _ => "file:/".to_owned(),
        };
        if ! path.ends_with('/') {
            path.push('/');
        }

        println!("main path: {}", path);
        self.set_path(&path);
        self.draw_content();
        'events: loop {
            let mut redraw = false;
            for event in self.event_loop() {
                match event {
                    FileManagerCommand::ChangeDir(dir) => {
                        self.selected = 0;
                        self.set_path(&dir);
                    }
                    FileManagerCommand::Execute(cmd) => {
                        Command::new(LAUNCH_COMMAND).arg(&cmd).spawn().unwrap();
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
