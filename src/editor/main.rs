#![deny(warnings)]

extern crate orbtk;

use orbtk::{Action, Button, Menu, Placeable, Point, Rect, TextBox, Window};
use orbtk::callback::Click;

use std::env;
use std::fs::File;
use std::io::{Read, Write};

fn main(){
    let path_option = env::args().nth(1);

    let title = if let Some(ref path) = path_option {
        format!("{} - Editor", path)
    } else {
        format!("Editor")
    };

    let width = 800;
    let height = 600;

    let mut window = Window::new(Rect::new(100, 100, width, height), &title);

    let text_box = TextBox::new()
        .position(0, 16)
        .size(width, height - 16)
        .place(&window);

    if let Some(ref path) = path_option {
        match File::open(path) {
            Ok(mut file) => {
                let mut text = String::new();
                match file.read_to_string(&mut text) {
                    Ok(_) => text_box.text.set(text),
                    Err(err) => println!("Failed to read {}: {}", path, err)
                }
            },
            Err(err) => println!("Failed to open {}: {}", path, err)
        }
    }

    let mut menu = Menu::new("File").position(0, 0).size(32, 16);

    menu.add_action(Action::new("Open").on_click(|_action: &Action, _point: Point| {
        println!("Open");
    }));

    menu.add_separator();

    let save_path_option = path_option.clone();
    let save_text_box = text_box.clone();
    menu.add_action(Action::new("Save").on_click(move |_action: &Action, _point: Point| {
        println!("Save");
        if let Some(ref path) = save_path_option {
            println!("Create {}", path);
            match File::create(path) {
                Ok(mut file) => {
                    let text = save_text_box.text.borrow();
                    match file.write(&text.as_bytes()) {
                        Ok(_) => match file.set_len(text.len() as u64) {
                            Ok(_) => println!("Successfully saved {}", path),
                            Err(err) => println!("Failed to truncate {}: {}", path, err)
                        },
                        Err(err) => println!("Failed to write {}: {}", path, err)
                    }
                },
                Err(err) => println!("Failed to open {}: {}", path, err)
            }
        } else {
            println!("Need to create file!");
        }
    }));

    let save_as_path_option = path_option.clone();
    menu.add_action(Action::new("Save As").on_click(move |_action: &Action, _point: Point| {
        println!("Save As");
        let mut window = Window::new(Rect::new(100, 100, 576, 32), "Save As");

        let text_box = TextBox::new()
            .position(0, 0)
            .size(576, 16)
            .place(&window);

        if let Some(ref path) = save_as_path_option {
            text_box.text.set(path.clone());
        }

        let window_cancel = &mut window as *mut Window;
        Button::new()
            .position(0, 16)
            .size(576/2, 16)
            .text("Cancel")
            .on_click(move |_button: &Button, _point: Point| {
                unsafe { (&mut *window_cancel).close(); }
            })
            .place(&window);

        let window_save_as = &mut window as *mut Window;
        Button::new()
            .position(576/2, 16)
            .size(576/2, 16)
            .text("Save As")
            .on_click(move |_button: &Button, _point: Point| {
                println!("Save {}", text_box.text.get());
                unsafe { (&mut *window_save_as).close(); }
            })
            .place(&window);

        window.exec();
    }));

    menu.add_separator();

    let window_close = &mut window as *mut Window;
    menu.add_action(Action::new("Close").on_click(move |_action: &Action, _point: Point| {
        println!("Close");
        unsafe { (&mut *window_close).close(); }
    }));

    menu.place(&window);

    window.exec();
}
