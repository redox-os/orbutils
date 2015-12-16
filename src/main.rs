extern crate orbtk;

use orbtk::*;

use std::env;
use std::fs::File;
use std::io::Read;

fn real_main(){
    let path_option = env::args().nth(1);

    let title = if let Some(ref path) = path_option {
        format!("{} - Editor", path)
    } else {
        format!("Editor")
    };

    let mut window = Window::new(Rect::new(100, 100, 420, 420), &title);

    let text_box = TextBox::new()
        .position(0, 16)
        .size(420, 404)
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

#[cfg(target_os = "redox")]
#[no_mangle]
pub fn main() {
    real_main();
}

#[cfg(not(target_os = "redox"))]
fn main() {
    real_main();
}
