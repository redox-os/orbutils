use std::{fs, path::PathBuf, process::Command};

#[derive(Clone)]
pub struct KeymapState {
    pub active: String,
    pub list: Vec<String>,
}

impl KeymapState {
    pub fn new() -> Self {
        let active = "us".to_string();
        let mut list = vec!["us".to_string()];

        if let Ok(output) = Command::new("inputd").arg("--keymaps").output() {
            list = String::from_utf8_lossy(&output.stdout[..])
                .trim()
                .split("\n")
                .map(|s| s.to_string())
                .collect();
            // TODO: Get active keymap
        }

        KeymapState { active, list }
    }

    pub fn set_active(&mut self, val: &str) -> bool {
        if val == self.active {
            return false;
        }

        if let Ok(output) = Command::new("inputd").arg("-K").arg(val).status() {
            if output.success() {
                self.active = val.to_owned();
                return true;
            }
        }

        return false;
    }
}
