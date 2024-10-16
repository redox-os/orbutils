use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use orbimage::Image;

use super::{load_icon, load_icon_small, load_icon_svg};

#[derive(Clone, Debug)]
pub enum IconSource {
    None,
    Name(String),
    Path(PathBuf),
}

impl IconSource {
    pub fn lookup(&mut self, small: bool) -> Option<&Path> {
        match self {
            IconSource::Name(name) => {
                let size = if small { 32 } else { 48 };
                let scale = crate::SCALE.load(Ordering::Relaxed) as u16;
                if let Some(path) = freedesktop_icons::lookup(name)
                    .with_size(size)
                    .with_scale(scale)
                    .with_theme("Cosmic")
                    .find()
                {
                    *self = IconSource::Path(path)
                } else {
                    log::warn!("failed to find icon {name} with size {size} and scale {scale}");
                }
            }
            _ => {}
        }
        match self {
            IconSource::Path(path) => Some(path),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Icon {
    pub source: IconSource,
    small: bool,
    image_opt: Option<Image>,
}

impl Icon {
    pub fn empty(small: bool) -> Self {
        Self {
            source: IconSource::None,
            small,
            image_opt: None,
        }
    }

    pub fn image(&mut self) -> &Image {
        if self.image_opt.is_none() {
            log::info!("loading {:?}", self.source);
            self.image_opt = if let Some(path) = self.source.lookup(self.small) {
                if path.extension() == Some(OsStr::new("png")) {
                    Some(if self.small {
                        load_icon_small(&path)
                    } else {
                        load_icon(&path)
                    })
                } else if path.extension() == Some(OsStr::new("svg")) {
                    load_icon_svg(&path, self.small)
                } else {
                    None
                }
            } else {
                None
            };
            if self.image_opt.is_none() {
                println!("failed to load icon {:?}", self.source);
                self.image_opt = Some(Image::default());
            }
        }
        self.image_opt.as_ref().unwrap()
    }
}

/// A package (_REDOX content serialized)
#[derive(Clone)]
pub struct Package {
    /// The ID of the package
    pub id: String,
    /// The name of the package
    pub name: String,
    /// The categories for the package
    pub categories: BTreeSet<String>,
    /// The exec string for the package, parsed by shlex
    pub exec: String,
    /// The icon for the package
    pub icon: Icon,
    /// A smaller icon for the package
    pub icon_small: Icon,
    /// The accepted extensions
    pub accepts: Vec<String>,
    /// The author(s) of the package
    pub authors: Vec<String>,
    /// The description of the package
    pub descriptions: Vec<String>,
}

impl Package {
    pub fn new() -> Self {
        Package {
            id: String::new(),
            name: String::new(),
            categories: BTreeSet::new(),
            exec: String::new(),
            icon: Icon::empty(false),
            icon_small: Icon::empty(true),
            accepts: Vec::new(),
            authors: Vec::new(),
            descriptions: Vec::new(),
        }
    }

    /// Create package from URL
    pub fn from_path(path: &str) -> Self {
        let mut package = Package::new();

        for part in path.rsplit('/') {
            if !part.is_empty() {
                package.id = part.to_string();
                break;
            }
        }

        let mut info = String::new();

        if let Ok(mut file) = File::open(path) {
            let _ = file.read_to_string(&mut info);
        }

        for line in info.lines() {
            if line.starts_with("name=") {
                package.name = line[5..].to_string();
            } else if line.starts_with("category=") {
                let category = &line[9..];
                if !category.is_empty() {
                    package.categories.insert(category.into());
                }
            } else if line.starts_with("binary=") {
                match shlex::try_quote(&line[7..]) {
                    Ok(binary) => {
                        // This adds %f to the binary for use in launching files
                        package.exec = format!("{binary} %f");
                    }
                    Err(err) => {
                        log::error!("failed to parse package info: {:?}: {}", line, err);
                    }
                }
            } else if line.starts_with("icon=") {
                let path = Path::new(&line[5..]);
                package.icon.source = IconSource::Path(path.into());
                package.icon_small.source = IconSource::Path(path.into());
            } else if line.starts_with("accept=") {
                package.accepts.push(line[7..].to_string());
            } else if line.starts_with("author=") {
                package.authors.push(line[7..].to_string());
            } else if line.starts_with("description=") {
                package.descriptions.push(line[12..].to_string());
            } else {
                log::error!("unknown package info: {}", line);
            }
        }

        package
    }

    pub fn from_desktop_entry(id: String, path: &Path) -> Option<Self> {
        let entry = freedesktop_entry_parser::parse_entry(path).ok()?;
        let mut package = Package::new();
        package.id = id;
        let section = entry.section("Desktop Entry");
        if let Some(name) = section.attr("Name") {
            package.name = name.into();
        }
        if let Some(categories) = section.attr("Categories") {
            // From https://specifications.freedesktop.org/menu-spec/latest/category-registry.html#main-category-registry
            let main_categories = [
                "AudioVideo",
                "Audio",
                "Video",
                "Development",
                "Education",
                "Game",
                "Graphics",
                "Network",
                "Office",
                "Science",
                "Settings",
                "System",
                "Utility",
            ];
            for category in categories.split_terminator(';') {
                if main_categories.contains(&category) {
                    // Some categories are renamed here
                    package.categories.insert(
                        match category {
                            "AudioVideo" | "Audio" | "Video" => "Multimedia",
                            "Game" => "Games",
                            _ => category,
                        }
                        .into(),
                    );
                }
            }
        }
        if let Some(exec) = section.attr("Exec") {
            package.exec = exec.into();
        }
        if let Some(icon) = section.attr("Icon") {
            package.icon.source = IconSource::Name(icon.into());
            package.icon_small.source = IconSource::Name(icon.into());
        }
        Some(package)
    }
}
