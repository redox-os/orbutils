use std::fs::File;
use std::io::Read;
use log::error;

use orbimage::Image;

use super::{load_icon, load_icon_small};

/// A package (_REDOX content serialized)
#[derive(Clone)]
pub struct Package {
    /// The ID of the package
    pub id: String,
    /// The name of the package
    pub name: String,
    /// The category for the package
    pub category: String,
    /// The binary for the package
    pub binary: String,
    /// The icon for the package
    pub icon: Image,
    /// A smaller icon for the package
    pub icon_small: Image,
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
            category: String::new(),
            binary: String::new(),
            icon: Image::default(),
            icon_small: Image::default(),
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
                package.category = line[9..].to_string();
            } else if line.starts_with("binary=") {
                package.binary = line[7..].to_string();
            } else if line.starts_with("icon=") {
                package.icon = load_icon(&line[5..]);
                package.icon_small = load_icon_small(&line[5..]);
            } else if line.starts_with("accept=") {
                package.accepts.push(line[7..].to_string());
            } else if line.starts_with("author=") {
                package.authors.push(line[7..].to_string());
            } else if line.starts_with("description=") {
                package.descriptions.push(line[12..].to_string());
            } else {
                error!("Unknown package info: {}", line);
            }
        }

        package
    }
}
