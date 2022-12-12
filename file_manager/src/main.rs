#[allow(clippy::all)]
mod generated_code {
    slint::include_modules!();
}

pub use generated_code::*;


pub fn main() {
    slint_orbclient::init_config(
        slint_orbclient::Config::default()
            .width(600)
            .height(400)
            .resizable(true)
            .events_async(true)
            .title("File Manager"),
    );

    let app = App::new();
    app.global::<coop>().set_embedded_helper(true);

    app.run();
}
