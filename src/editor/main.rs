#![deny(warnings)]

extern crate orbtk;

use orbtk::{Action, Button, Menu, Placeable, Point, Rect, TextBox, Window};
use orbtk::callback::Click;

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Arc;

fn main(){
    let path_option = Arc::new(env::args().nth(1));

    let title = if let Some(ref path) = *path_option {
        format!("{} - Editor", path)
    } else {
        format!("Editor")
    };

    let window = Arc::new(Window::new(Rect::new(100, 100, 576, 420), &title));

    let text_box = TextBox::new()
        .position(0, 16)
        .size(576, 404)
        .place(&window);

    if let Some(ref path) = *path_option {
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
    menu.add_action(Action::new("Save").on_click(move |_action: &Action, _point: Point| {
        println!("Save");
        if let Some(ref path) = *save_path_option {
            match File::create(path) {
                Ok(mut file) => {
                    let text = text_box.text.get();
                    match file.write(&mut text.as_bytes()) {
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
        let window = Arc::new(Window::new(Rect::new(100, 100, 576, 32), "Save As"));

        let text_box = TextBox::new()
            .position(0, 0)
            .size(576, 16)
            .place(&window);

        if let Some(ref path) = *save_as_path_option {
            text_box.text.set(path.clone());
        }

        let window_cancel = window.clone();
        Button::new()
            .position(0, 16)
            .size(576/2, 16)
            .text("Cancel")
            .on_click(move |_button: &Button, _point: Point| {
                window_cancel.close();
            })
            .place(&window);

        let window_save_as = window.clone();
        Button::new()
            .position(576/2, 16)
            .size(576/2, 16)
            .text("Save As")
            .on_click(move |_button: &Button, _point: Point| {
                println!("Save {}", text_box.text.get());
                window_save_as.close();
            })
            .place(&window);

        window.exec();
    }));

    menu.add_separator();

    let window_close = window.clone();
    menu.add_action(Action::new("Close").on_click(move |_action: &Action, _point: Point| {
        println!("Close");
        window_close.close();
    }));

    menu.place(&window);

    window.exec();
}
