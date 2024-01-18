extern crate orbclient;
extern crate orbimage;
extern crate orbtk;
extern crate mime_guess;
extern crate mime;
extern crate dirs;
extern crate redox_log;
extern crate log;

use std::{cmp, env, fs};
use std::collections::BTreeMap;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::{String, ToString};
use std::vec::Vec;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Arc;
use log::{debug, error, info};

use mime::TopLevel as MimeTop;

use orbclient::{Color, Renderer, WindowFlag};
use orbimage::Image;

use orbtk::{Window, WindowBuilder, Point, Rect, Button, List, Entry, Label, Place, Resize, Text, Style, TextBox, Click, Enter};
use orbtk::theme::Theme;
use redox_log::{OutputBuilder, RedoxLogger};

const ICON_SIZE: i32 = 32;

#[cfg(target_os = "redox")]
static UI_PATH: &'static str = "/ui/icons";

#[cfg(not(target_os = "redox"))]
static UI_PATH: &'static str = "ui/icons";

#[cfg(target_os = "redox")]
static LAUNCH_COMMAND: &'static str = "launcher";

#[cfg(not(target_os = "redox"))]
static LAUNCH_COMMAND: &'static str = "xdg-open";

static FILE_MANAGER_THEME_CSS: &'static str = include_str!("theme.css");

struct FileInfo {
    name: String,
    path: PathBuf,
    size: u64,
    size_str: String,
    is_dir: bool,
}

impl FileInfo {
    fn new<P: AsRef<Path>>(name: String, path: P, is_dir: bool) -> FileInfo {
        let (size, size_str) = {
            if is_dir {
                FileManager::get_num_entries(path.as_ref())
            } else {
                match fs::metadata(path.as_ref()) {
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
            name,
            path: path.as_ref().to_owned(),
            size,
            size_str,
            is_dir,
        }
    }
}

struct FileType {
    description: String,
    icon: PathBuf
}

impl FileType {
    fn new(desc: String, icon: &'static str) -> Option<Self> {
        for folder in ["mimetypes", "places"].iter() {
            match fs::canonicalize(UI_PATH) {
                Ok(mut path) => {
                    path.push(folder);
                    path.push(format!("{}.png", icon));
                    if path.is_file() {
                        return Some(Self {
                            description: desc,
                            icon: path,
                        });
                    } else {
                        error!("{} not found in {}", icon, folder);
                    }
                },
                Err(err) => error!("failed to canonicalize {}: {}", UI_PATH, err)
            }
        }

        error!("{} not found", icon);
        match fs::canonicalize(UI_PATH) {
            Ok(mut path) => {
                path.push("mimetypes/unknown.png");
                Some(Self {
                    description: desc,
                    icon: path,
                })
            },
            Err(err) => {
                error!("failed to canonicalize {}: {}", UI_PATH, err);
                None
            }
        }
    }

    fn from_filename(file_name: &str) -> Option<Self> {
        if file_name.ends_with('/') {
            Self::new("folder".to_owned(), "inode-directory")
        } else {
            let ext = file_name.rsplitn(2, '.').next().unwrap_or("");
            let mime = mime_guess::get_mime_type(ext);
            let image = match (&mime.0, &mime.1) {
                (&MimeTop::Image, _) => "image-x-generic",
                (&MimeTop::Text, _) => "text-plain",
                (&MimeTop::Audio, _) => "audio-x-generic",
                _ => match ext {
                    "c" | "cpp" | "h" => "text-x-c",
                    "asm" | "ion" | "lua" | "rc" | "rs" | "sh" => "text-x-script",
                    "ttf" => "application-x-font-ttf",
                    "tar" => "package-x-generic",
                    _ => "unknown"
                }
            };
            Self::new(format!("{}", mime), image)
        }
    }
}

struct FileTypesInfo {
    empty_image: Image,
    images: BTreeMap<PathBuf, Image>,
}

impl FileTypesInfo {
    pub fn new() -> Self {
        Self {
            empty_image: Image::new(0, 0),
            images: BTreeMap::new()
        }
    }

    pub fn description_for(&self, file_name: &str) -> String {
        match FileType::from_filename(file_name) {
            Some(file_type) => file_type.description,
            None => String::new()
        }
    }

    pub fn icon_for(&mut self, file_name: &str) -> &Image {
        match FileType::from_filename(file_name) {
            Some(file_type) => {
                let icon = file_type.icon;
                if ! self.images.contains_key(&icon) {
                    self.images.insert(icon.clone(), load_icon(&icon));
                }
                &self.images[&icon]
            },
            None => {
                &self.empty_image
            }
        }
    }
}

enum FileManagerCommand {
    ChangeDir(PathBuf),
    Execute(PathBuf),
    CreateFile(String),
    CreateFolder(String),
    ChangeSort(usize),
    Resize(u32, u32),
}

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone, Copy)]
struct Column {
    name: &'static str,
    x: i32,
    width: i32,
    sort_predicate: SortPredicate,
}

pub struct FileManager {
    path: PathBuf,
    file_types_info: FileTypesInfo,
    files: Vec<FileInfo>,
    columns: [Column; 3],
    column_labels: Vec<Arc<Label>>,
    sort_predicate: SortPredicate,
    sort_direction: SortDirection,
    window: Window,
    window_width: u32,
    window_height: u32,
    list_widget_index: Option<usize>,
    tx: Sender<FileManagerCommand>,
    rx: Receiver<FileManagerCommand>,
}

fn load_icon(path: &Path) -> Image {
    match Image::from_path(path) {
        Ok(icon) => if icon.width() == ICON_SIZE as u32 && icon.height() == ICON_SIZE as u32 {
            icon
        } else {
            icon.resize(ICON_SIZE as u32, ICON_SIZE as u32, orbimage::ResizeType::Lanczos3).unwrap()
        },
        Err(err) => {
            error!("Failed to load icon {}: {}", path.display(), err);
            Image::from_color(ICON_SIZE as u32, ICON_SIZE as u32, Color::rgba(0, 0, 0, 0))
        }
    }
}

impl FileManager {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let (tx, rx) = channel();

        let (display_width, display_height) = orbclient::get_display_size().expect("viewer: failed to get display size");
        let (window_w, window_h) = (cmp::min(640, display_width * 4/5) as i32, cmp::min(480, display_height * 4/5) as i32);

        let theme = Theme::parse(FILE_MANAGER_THEME_CSS);

        let mut window_builder = WindowBuilder::new(Rect::new(-1, -1, window_w as u32, window_h as u32), "File Manager")
            .theme(theme)
            .flags(&[WindowFlag::Resizable]);
        window_builder = window_builder;
        let window = window_builder.build();

        let tx_resize = tx.clone();
        window.on_resize(move |_window, width, height| {
            tx_resize.send(FileManagerCommand::Resize(width, height)).unwrap();
        });

        FileManager {
            path: match fs::canonicalize(path.as_ref()) {
                Ok(p) => p,
                _ => PathBuf::from("/"),
            },
            file_types_info: FileTypesInfo::new(),
            files: Vec::new(),
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
            column_labels: Vec::new(),
            sort_predicate: SortPredicate::Name,
            sort_direction: SortDirection::Asc,
            window: window,
            window_width: window_w as u32,
            window_height: window_h as u32,
            list_widget_index: None,
            tx: tx,
            rx: rx,
        }
    }

    fn get_num_entries<P: AsRef<Path>>(path: P) -> (u64, String) {
        let count = match fs::read_dir(path.as_ref()) {
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

    fn resized_columns(&self) -> [Column; 3] {
        let mut columns = self.columns;
        columns[0].width = cmp::max(
            columns[0].width,
            self.window_width as i32
            - columns[0].x
            - columns[1].width
            - columns[2].width
        );
        columns[1].x = columns[0].x + columns[0].width;
        columns[2].x = columns[1].x + columns[1].width;
        columns
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

    fn update_headers(&mut self) {
        if self.column_labels.get(0).is_none() {
            let label = Label::new();
            self.window.add(&label);
            label.with_class("entry");
            label.text_offset.set(Point::new(0, 8));

            let tx = self.tx.clone();
            let self_ptr = self as *mut FileManager;
            label.on_click(move |_, _| {
                // DESIGN {
                let (p_x, p_y, p_w, p_h) = {
                    let s = unsafe { &mut *self_ptr };
                    (s.window.x(), s.window.y(), s.window.width(), s.window.height())
                };

                let w = 320;
                let h = 8 + 28 + 8 + 28 + 8;
                let x = p_x + (p_w as i32 - w as i32)/2;
                let y = p_y + (p_h as i32 - h as i32)/2;

                let mut window = Box::new(Window::new(Rect::new(x, y, w, h), "New"));

                let text_box = TextBox::new();
                text_box.position(8, 8)
                    .size(w - 16, 28)
                    .text_offset(6, 6)
                    .grab_focus(true);
                window.add(&text_box);

                let cancel_button = Button::new();
                cancel_button.position(8, 8 + 28 + 8)
                    .size((w - 16)/3 - 4, 28)
                    .text_offset(6, 6)
                    .text("Cancel");
                window.add(&cancel_button);

                let folder_button = Button::new();
                folder_button.position((w as i32)/3 + 4, 8 + 28 + 8)
                    .size((w - 16)/3 - 4, 28)
                    .text_offset(6, 6)
                    .text("Folder");
                window.add(&folder_button);

                let file_button = Button::new();
                file_button.position((w as i32) * 2/3 + 4, 8 + 28 + 8)
                    .size((w - 16)/3 - 4, 28)
                    .text_offset(6, 6)
                    .text("File");
                window.add(&file_button);
                // } DESIGN

                {
                    let text_box = text_box.clone();
                    let tx = tx.clone();
                    let window_ptr = window.deref_mut() as *mut Window;
                    text_box.on_enter(move |text_box: &TextBox| {
                        let name = text_box.text.get();
                        if ! name.is_empty() {
                            tx.send(FileManagerCommand::CreateFile(name)).unwrap();
                        }
                        unsafe { (&*window_ptr).close(); }
                    });
                }

                {
                    let window_ptr = window.deref_mut() as *mut Window;
                    cancel_button.on_click(move |_button: &Button, _point: Point| {
                        unsafe { (&*window_ptr).close(); }
                    });
                }

                {
                    let text_box = text_box.clone();
                    let tx = tx.clone();
                    let window_ptr = window.deref_mut() as *mut Window;
                    file_button.on_click(move |_button: &Button, _point: Point| {
                        let name = text_box.text.get();
                        if ! name.is_empty() {
                            tx.send(FileManagerCommand::CreateFile(name)).unwrap();
                        }
                        unsafe { (&*window_ptr).close(); }
                    });
                }

                {
                    let text_box = text_box.clone();
                    let tx = tx.clone();
                    let window_ptr = window.deref_mut() as *mut Window;
                    folder_button.on_click(move |_button: &Button, _point: Point| {
                        let name = text_box.text.get();
                        if ! name.is_empty() {
                            tx.send(FileManagerCommand::CreateFolder(name)).unwrap();
                        }
                        unsafe { (&*window_ptr).close(); }
                    });
                }

                window.exec();
            });
            self.column_labels.push(label);
        }

        if let Some(label) = self.column_labels.get(0) {
            label.position(16, 0).size(8, 32).text("+".to_string());
        }

        let columns = self.resized_columns();
        for (i, column) in columns.iter().enumerate() {
            if self.column_labels.get(i * 2 + 1).is_none() {
                // header text
                let label = Label::new();
                self.window.add(&label);
                label.with_class("header");
                label.text_offset.set(Point::new(0, 8));

                let tx = self.tx.clone();
                label.on_click(move |_, _| {
                    tx.send(FileManagerCommand::ChangeSort(i)).unwrap();
                });
                self.column_labels.push(label);

                // sort arrow
                let label = Label::new();
                self.window.add(&label);
                label.with_class("sort");
                label.text_offset.set(Point::new(0, 8));
                self.column_labels.push(label);
            }

            if let Some(label) = self.column_labels.get(i * 2 + 1) {
                label.position(column.x, 0).size(column.width as u32, 32).text(column.name);
            }

            if let Some(label) = self.column_labels.get(i * 2 + 2) {
                if column.sort_predicate == self.sort_predicate {
                    let arrow = match self.sort_direction {
                        SortDirection::Asc => "↓",
                        SortDirection::Desc => "↑",
                    };

                    label.position(column.x + column.width - 12, 0).size(16, 32).text(arrow);
                } else {
                    label.text("");
                }
            }
        }
    }

    fn update_list(&mut self) {
        let columns = self.resized_columns();
        let w = (columns[2].x + columns[2].width) as u32;

        let list = List::new();
        list.position(0, 32).size(w, self.window_height - 32);

        {
            for file in self.files.iter() {
                let entry = Entry::new(ICON_SIZE as u32);

                let path = file.path.clone();
                let tx = self.tx.clone();

                entry.on_click(move |_, _| {
                    if path.is_dir() {
                        tx.send(FileManagerCommand::ChangeDir(path.clone())).unwrap();
                    } else {
                        tx.send(FileManagerCommand::Execute(path.clone())).unwrap();
                    }
                });

                {
                    let icon = self.file_types_info.icon_for(&file.name);
                    let image = orbtk::Image::from_image((*icon).clone());
                    image.position(4, 0);
                    entry.add(&image);
                }

                let mut label = Label::new();
                label.position(columns[0].x, 0).size(w, ICON_SIZE as u32).text(file.name.clone());
                label.text_offset.set(Point::new(0, 8));
                label.with_class("file-name");
                entry.add(&label);

                label = Label::new();
                label.position(columns[1].x, 0).size(w, ICON_SIZE as u32).text(file.size_str.clone());
                label.text_offset.set(Point::new(0, 8));
                label.with_class("file-size");
                entry.add(&label);

                let description = self.file_types_info.description_for(&file.name);
                label = Label::new();
                label.position(columns[2].x, 0).size(w, ICON_SIZE as u32).text(description);
                label.text_offset.set(Point::new(0, 8));
                label.with_class("description");
                entry.add(&label);

                list.push(&entry);
            }
        }

        if let Some(i) = self.list_widget_index {
            let mut widgets = self.window.widgets.borrow_mut();
            widgets.remove(i);
            widgets.insert(i, list);
        } else {
            self.list_widget_index = Some(self.window.add(&list));
        }
    }

    fn update_path(&mut self) {
        self.window.set_title(&format!("{}", self.path.display()));

        for column in self.columns.iter_mut() {
            column.width = (column.name.len() * 8) as i32 + 16;
        }

        self.files.clear();

        // check to see if parent directory exists
        if let Some(parent) = self.path.parent().map(|p| p.to_owned()) {
            self.push_file(FileInfo::new("../".to_string(), parent, true));
        }

        match fs::read_dir(&self.path) {
            Ok(readdir) => {
                for entry_result in readdir {
                    match entry_result {
                        Ok(entry) => {
                            let directory = match entry.file_type() {
                                Ok(file_type) => file_type.is_dir(),
                                Err(err) => {
                                    error!("Failed to read file type: {}", err);
                                    false
                                }
                            };

                            let name = match entry.file_name().to_str() {
                                Some(name) => if directory {
                                    name.to_string() + "/"
                                } else {
                                    name.to_string()
                                },
                                None => {
                                    error!("Failed to read file name");
                                    String::new()
                                }
                            };

                            self.push_file(FileInfo::new(name, entry.path(), directory));
                        },
                        Err(err) => error!("failed to read dir entry: {}", err)
                    }
                }
            },
            Err(err) => error!("failed to readdir {}: {}", self.path.display(), err)
        }

        self.columns[0].x = ICON_SIZE + 8;
        self.columns[1].x = self.columns[0].x + self.columns[0].width;
        self.columns[2].x = self.columns[1].x + self.columns[1].width;

        self.sort_files();
    }

    fn redraw(&mut self) {
        self.update_headers();
        self.update_list();

        self.window.needs_redraw();
    }

    fn exec(&mut self) {
        debug!("main path: {}", self.path.display());
        self.update_path();
        self.redraw();
        self.window.draw_if_needed();

        while self.window.running.get() {
            self.window.step();

            while let Ok(event) = self.rx.try_recv() {
                match event {
                    FileManagerCommand::ChangeDir(dir) => {
                        self.path = dir;
                        self.update_path();
                        self.redraw();
                    }
                    FileManagerCommand::Execute(cmd) => {
                        Command::new(LAUNCH_COMMAND).arg(&cmd).spawn().unwrap();
                    },
                    FileManagerCommand::CreateFile(name) => {
                        let path = self.path.join(name);
                        match fs::File::create(&path) {
                            Ok(_) => {
                                info!("Created file {}", path.display());
                            },
                            Err(err) => {
                                error!("Failed to create file {}: {}", path.display(), err);
                            }
                        }
                        self.update_path();
                        self.redraw();
                    },
                    FileManagerCommand::CreateFolder(name) => {
                        let path = self.path.join(name);
                        match fs::create_dir(&path) {
                            Ok(_) => info!("Created folder {}", path.display()),
                            Err(err) => error!("Failed to create folder {}: {}", path.display(), err)
                        }
                        self.update_path();
                        self.redraw();
                    },
                    FileManagerCommand::ChangeSort(i) => {
                        let predicate = match i {
                            0 => SortPredicate::Name,
                            1 => SortPredicate::Size,
                            2 => SortPredicate::Type,
                            _ => return
                        };

                        if self.sort_predicate != predicate {
                            self.sort_predicate = predicate;
                        } else {
                            self.sort_direction.invert();
                        }

                        self.sort_files();

                        self.redraw();
                    },
                    FileManagerCommand::Resize(width, height) => {
                        self.window_width = width;
                        self.window_height = height;

                        self.redraw();
                    }
                }
            }

            self.window.draw_if_needed();
        }
    }
}

fn main() {
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("file_manager".into())
        .enable();

    match env::args().nth(1) {
        Some(ref arg) => FileManager::new(arg).exec(),
        None => if let Some(home) = dirs::home_dir() {
            FileManager::new(home).exec()
        } else {
            FileManager::new(".").exec()
        }
    }
}
