use std::fs::File;
use std::io::Read;

use orbimage::Image;

/// A package (_REDOX content serialized)
pub struct Package {
    /// The URL
    pub url: String,
    /// The ID of the package
    pub id: String,
    /// The name of the package
    pub name: String,
    /// The binary for the package
    pub binary: String,
    /// The icon for the package
    pub icon: Image,
    /// The accepted extensions
    pub accepts: Vec<String>,
    /// The author(s) of the package
    pub authors: Vec<String>,
    /// The description of the package
    pub descriptions: Vec<String>,
}

impl Package {
    /// Create package from URL
    pub fn from_path(url: &str) -> Self {
        let mut package = Package {
            url: url.to_string(),
            id: String::new(),
            name: String::new(),
            binary: String::new(),
            icon: Image::default(),
            accepts: Vec::new(),
            authors: Vec::new(),
            descriptions: Vec::new(),
        };

        for part in url.rsplit('/') {
            if !part.is_empty() {
                package.id = part.to_string();
                break;
            }
        }

        let mut info = String::new();

        if let Ok(mut file) = File::open(url) {
            let _ = file.read_to_string(&mut info);
        }

        for line in info.lines() {
            if line.starts_with("name=") {
                package.name = line[5..].to_string();
            } else if line.starts_with("binary=") {
                package.binary = line[7..].to_string();
            } else if line.starts_with("icon=") {
                package.icon = Image::from_path(&line[5..]).unwrap_or(Image::default());
            } else if line.starts_with("accept=") {
                package.accepts.push(line[7..].to_string());
            } else if line.starts_with("author=") {
                package.authors.push(line[7..].to_string());
            } else if line.starts_with("description=") {
                package.descriptions.push(line[12..].to_string());
            } else {
                println!("Unknown package info: {}", line);
            }
        }

        package
    }
}
