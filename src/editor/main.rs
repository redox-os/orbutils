#![deny(warnings)]

extern crate orbtk;

use orbtk::{Action, Menu, Point, Rect, TextBox, Window};
use orbtk::callback::Click;
use orbtk::place::Place;

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

    let mut window = Window::new(Rect::new(100, 100, 576, 420), &title);

    let text_box = TextBox::new()
        .position(0, 16)
        .size(576, 404)
        .place(&mut window);

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

    menu.add_action(Action::new("Save").on_click(move |_action: &Action, _point: Point| {
        println!("Save");
        if let Some(ref path) = path_option {
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

    menu.add_action(Action::new("Save As").on_click(|_action: &Action, _point: Point| {
        println!("Save As");
    }));

    menu.add_separator();

    menu.add_action(Action::new("Close").on_click(|_action: &Action, _point: Point| {
        println!("Close");
    }));

    menu.place(&mut window);

    window.exec();
}
