use std::{fs, path::PathBuf};

#[derive(Clone)]
pub struct KeymapState {
    pub active: String,
    pub list: Vec<String>,
    pub devices: Vec<PathBuf>,
}

impl KeymapState {
    pub fn new() -> Self {
        let mut devices = Vec::new();
        let mut active = "us".to_string();
        let mut list = vec!["us".to_string()];

        //TODO: usbhidd
        for device in vec![PathBuf::from("/scheme/ps2")] {
            if !device.is_dir() {
                continue;
            }
            if let Ok(s) = fs::read_to_string(device.join("keymap")) {
                active = s.trim().to_string();
            }
            if let Ok(s) = fs::read_to_string(device.join("keymap_list")) {
                list = s.trim().split("\n").map(|s| s.to_string()).collect();
                //FIXME: why?
                list.pop();
            }
            devices.push(device);
        }

        KeymapState {
            active,
            list,
            devices,
        }
    }
    pub fn is_available(&self) -> bool {
        self.devices.len() > 0
    }
    pub fn set_active(&mut self, val: &str) -> bool {
        if !self.is_available() || val == self.active {
            return false;
        }
        for device in &self.devices {
            if fs::write(device.join("keymap"), val.as_bytes()).is_err() {
                return false;
            }
        }
        self.active = val.to_owned();
        return true;
    }
}
