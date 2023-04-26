extern crate orbclient;
extern crate orbtk;
extern crate redox_log;
extern crate log;

use orbclient::WindowFlag;
use orbtk::{Action, Button, Menu, Point, Rect, Separator, TextBox, Window};
use orbtk::dialogs::FileDialog;
use orbtk::traits::{Click, Enter, Place, Resize, Text};

use std::{cmp, env};
use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use log::{debug, error, info};
use redox_log::{OutputBuilder, RedoxLogger};

pub struct Editor {
    path_option: Option<PathBuf>,
    text_box: Arc<TextBox>,
    window: *mut Window,
}

impl Editor {
    pub fn new(path_option: Option<PathBuf>, width: u32, height: u32) -> Box<Window> {
        // DESIGN {
        let mut window =  Box::new(Window::new_flags(Rect::new(-1, -1, width, height), "Editor", &[WindowFlag::Resizable]));

        let text_box = TextBox::new();
        text_box.position(0, 16)
            .size(width, height - 16);
        window.add(&text_box);

        let menu = Menu::new("File");
        menu.position(4, 0).size(32, 16);

        let open_action = Action::new("Open");
        menu.add(&open_action);

        let reload_action = Action::new("Reload");
        menu.add(&reload_action);

        menu.add(&Separator::new());

        let save_action = Action::new("Save");
        menu.add(&save_action);

        let save_as_action = Action::new("Save As");
        menu.add(&save_as_action);

        menu.add(&Separator::new());

        let close_action = Action::new("Close");
        menu.add(&close_action);

        window.add(&menu);
        // } DESIGN

        // CODE {
        let editor_cell = Rc::new(RefCell::new(Editor {
            path_option: path_option,
            text_box: text_box.clone(),
            window: window.deref_mut() as *mut Window,
        }));

        {
            let mut editor = editor_cell.borrow_mut();
            if editor.path_option.is_some() {
                editor.load();
            }
        }

        {
            let editor_cell = editor_cell.clone();
            open_action.on_click(move |_action: &Action, _point: Point| {
                debug!("Open");

                let mut dialog = FileDialog::new();
                dialog.title = "Open File".to_string();
                if let Some(ref path) = editor_cell.borrow().path_option {
                    if let Ok(canon) = path.canonicalize() {
                        if let Some(parent) = canon.parent() {
                            dialog.path = parent.to_owned();
                        }
                    }
                }

                if let Some(path) = dialog.exec() {
                    debug!("Open {}", path.display());
                    editor_cell.borrow_mut().open(&path);
                }
            });
        }

        {
            let editor_cell = editor_cell.clone();
            reload_action.on_click(move |_action: &Action, _point: Point| {
                debug!("Reload");
                editor_cell.borrow_mut().load();
            });
        }

        {
            let editor_cell = editor_cell.clone();
            save_action.on_click(move |_action: &Action, _point: Point| {
                debug!("Save");
                editor_cell.borrow_mut().save();
            });
        }

        {
            let editor_cell = editor_cell.clone();
            save_as_action.on_click(move |_action: &Action, _point: Point| {
                debug!("Save As");

                let mut window = {
                    let editor_dialog = editor_cell.clone();
                    editor_cell.borrow_mut().path_dialog("Save As", move |path| {
                        debug!("Save As {}", path);
                        editor_dialog.borrow_mut().save_as(&path);
                    })
                };

                window.exec();
            });
        }

        {
            let editor_cell = editor_cell.clone();
            close_action.on_click(move |_action: &Action, _point: Point| {
                debug!("Close");
                editor_cell.borrow_mut().close();
            });
        }

        {
            let editor_cell = editor_cell.clone();
            window.on_resize(move |_, width, height| {
                editor_cell.borrow().text_box.size(width, height - 16);
            });
        }
        // } CODE

        window
    }

    fn path_dialog<F: Fn(&str) + 'static>(&mut self, title: &str, func: F) -> Box<Window> {
        let func_rc = Rc::new(func);

        // DESIGN {
        let (p_x, p_y, p_w, p_h) = {
            let window = unsafe { &mut *self.window };
            (window.x(), window.y(), window.width(), window.height())
        };

        let w = 320;
        let h = 8 + 28 + 8 + 28 + 8;
        let x = p_x + (p_w as i32 - w as i32)/2;
        let y = p_y + (p_h as i32 - h as i32)/2;

        let mut window = Box::new(Window::new(Rect::new(x, y, w, h), title));

        let text_box = TextBox::new();
        text_box.position(8, 8)
            .size(w - 16, 28)
            .text_offset(6, 6)
            .grab_focus(true);
        window.add(&text_box);

        let cancel_button = Button::new();
        cancel_button.position(8, 8 + 28 + 8)
            .size((w - 16)/2 - 4, 28)
            .text_offset(6, 6)
            .text("Cancel");
        window.add(&cancel_button);

        let confirm_button = Button::new();
        confirm_button.position((w as i32)/2 + 4, 8 + 28 + 8)
            .size((w - 16)/2 - 4, 28)
            .text_offset(6, 6)
            .text(title);
        window.add(&confirm_button);
        // } DESIGN

        // CODE {
        if let Some(ref path) = self.path_option {
            if let Some(path_str) = path.to_str() {
                text_box.text.set(path_str.to_string());
            }
        }

        {
            let func_rc = func_rc.clone();
            let window_ptr = window.deref_mut() as *mut Window;
            text_box.on_enter(move |text_box: &TextBox| {
                let path = text_box.text.get();
                func_rc(&path);
                unsafe { (&mut *window_ptr).close(); }
            });
        }

        {
            let window_ptr = window.deref_mut() as *mut Window;
            cancel_button.on_click(move |_button: &Button, _point: Point| {
                unsafe { (&mut *window_ptr).close(); }
            });
        }

        {
            let func_rc = func_rc.clone();
            let window_ptr = window.deref_mut() as *mut Window;
            confirm_button.on_click(move |_button: &Button, _point: Point| {
                let path = text_box.text.get();
                func_rc(&path);
                unsafe { (&mut *window_ptr).close(); }
            });
        }
        // } CODE

        window
    }

    fn load(&mut self) {
        if let Some(ref path) = self.path_option {
            debug!("Load {}", path.display());
            match File::open(path) {
                Ok(mut f) => {
                    let mut contents = String::new();
                    match f.read_to_string(&mut contents) {
                        Ok(_) => {
                            self.text_box.text.set(contents);
                        },
                        Err(e) => error!("Failed to read {}: {}", path.display(), e)
                    }
                },
                Err(e) => error!("Failed to open {}: {}", path.display(), e)
            }
        } else {
            error!("Path not set");
        }
    }

    fn save(&mut self) {
        if let Some(ref path) = self.path_option {
            info!("Save {}", path.display());
            match File::create(path) {
                Ok(mut file) => {
                    let text = self.text_box.text.borrow();
                    match file.write(&text.as_bytes()) {
                        Ok(_) => match file.set_len(text.len() as u64) {
                            Ok(_) => info!("Successfully saved {}", path.display()),
                            Err(err) => error!("Failed to truncate {}: {}", path.display(), err)
                        },
                        Err(err) => error!("Failed to write {}: {}", path.display(), err)
                    }
                },
                Err(err) => error!("Failed to open {}: {}", path.display(), err)
            }
        } else {
            error!("Path not set");
        }
    }

    fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path_option = Some(path.as_ref().to_owned());
        let window = unsafe { &mut *self.window };
        window.set_title(&format!("{} - Editor", path.as_ref().display()));
    }

    fn open<P: AsRef<Path>>(&mut self, path: P) {
        self.set_path(path);
        self.load();
    }

    fn save_as<P: AsRef<Path>>(&mut self, path: P) {
        self.set_path(path);
        self.save();
    }

    fn close(&mut self) {
        let window = unsafe { &mut *self.window };
        window.close();
    }
}

fn main(){
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Debug)
                .with_ansi_escape_codes()
                .build()
        )
        .with_process_name("editor".into())
        .enable();

    let path_option = env::args().nth(1).map(PathBuf::from);

    let (display_width, display_height) = orbclient::get_display_size().expect("viewer: failed to get display size");
    let (width, height) = (cmp::min(1024, display_width * 4/5), cmp::min(768, display_height * 4/5));

    Editor::new(path_option, width, height).exec();
}
