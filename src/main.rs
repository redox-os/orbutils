extern crate orbtk;

use orbtk::*;

fn real_main(){
    let mut window = Window::new(Rect::new(100, 100, 420, 420), "Editor");

    TextBox::new()
        .position(0, 0)
        .size(420, 420)
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
