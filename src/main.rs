extern crate orbtk;

#[cfg(target_os = "redox")]
#[no_mangle]
pub fn main() {
    orbtk::example();
}

#[cfg(not(target_os = "redox"))]
fn main() {
    orbtk::example();
}
