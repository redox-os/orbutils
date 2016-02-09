extern crate orbtk;

use orbtk::{Button, Point, Rect, TextBox, Window};
use orbtk::callback::Click;
use orbtk::place::Place;

use std::env;
use std::fs::File;
use std::io::Read;

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

    Button::new()
        .position(0, 0)
        .size(32, 16)
        .text("Save")
        .on_click(move |_button: &Button, _point: Point| {
            println!("{}", text_box.text.get());
        })
        .place(&mut window);

    window.exec();
}
