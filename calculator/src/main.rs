#[allow(clippy::all)]
mod generated_code {
    slint::include_modules!();
}

pub use generated_code::*;

use slint::SharedString;

fn eval(input: &str) -> String {
    match calc::eval(input) {
        Ok(s) => s.to_string(),
        Err(e) => e.into(),
    }
}

pub fn main() {
    let app = App::new().unwrap();
    app.global::<Theme>().set_embedded_helper(true);

    app.on_backspace(backspace);
    app.on_calculate(calculate);
    app.on_validate(validate);

    app.run().unwrap();
}

fn backspace(input: SharedString) -> SharedString {
    let mut input = input.to_string();
    input.pop();
    input.into()
}

fn calculate(input: SharedString) -> SharedString {
    eval(input.as_str()).into()
}

fn validate(input: SharedString) -> SharedString {
    let valid = match input.as_str() {
        "(" | ")" | "^" | "/" | "*" | "-" | "+" | "." | " " => true,
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => true,
        _ => false,
    };

    if valid {
        return input;
    }

    SharedString::default()
}

#[cfg(test)]
mod tests {
    use slint::platform::Key;

    use super::*;

    #[test]
    fn test_backspace() {
        assert_eq!(
            backspace(SharedString::from("5+3")),
            SharedString::from("5+")
        );
        assert_eq!(backspace(SharedString::from("5+")), SharedString::from("5"));
        assert_eq!(backspace(SharedString::from("5")), SharedString::from(""));
        assert_eq!(backspace(SharedString::from("")), SharedString::from(""));
    }

    #[test]
    fn test_calculate() {
        assert_eq!(
            calculate(SharedString::from("6/2")),
            SharedString::from("3")
        );
        assert_eq!(
            calculate(SharedString::from("5+3")),
            SharedString::from("8")
        );
        assert_eq!(
            calculate(SharedString::from("5 + 3")),
            SharedString::from("8")
        );
        assert_eq!(
            calculate(SharedString::from("(5 + 3)")),
            SharedString::from("8")
        );
        assert_eq!(
            calculate(SharedString::from("5*3")),
            SharedString::from("15")
        );
        assert_eq!(
            calculate(SharedString::from("5 * 3")),
            SharedString::from("15")
        );
        assert_eq!(
            calculate(SharedString::from("(5 * 3)")),
            SharedString::from("15")
        );
        assert_eq!(
            calculate(SharedString::from("5/2")),
            SharedString::from("2.5")
        );
        assert_eq!(
            calculate(SharedString::from("5 / 2")),
            SharedString::from("2.5")
        );
        assert_eq!(
            calculate(SharedString::from("(5 / 2)")),
            SharedString::from("2.5")
        );
        assert_eq!(
            calculate(SharedString::from("2*5+3")),
            SharedString::from("13")
        );
        assert_eq!(
            calculate(SharedString::from("2 * 5 + 3")),
            SharedString::from("13")
        );
        assert_eq!(
            calculate(SharedString::from("(2 * 5) + 3")),
            SharedString::from("13")
        );
    }

    #[test]
    fn test_validate() {
        assert_eq!(validate(SharedString::from("(")), SharedString::from("("));
        assert_eq!(validate(SharedString::from(")")), SharedString::from(")"));
        assert_eq!(validate(SharedString::from("^")), SharedString::from("^"));
        assert_eq!(validate(SharedString::from("/")), SharedString::from("/"));
        assert_eq!(validate(SharedString::from("*")), SharedString::from("*"));
        assert_eq!(validate(SharedString::from("-")), SharedString::from("-"));
        assert_eq!(validate(SharedString::from("+")), SharedString::from("+"));
        assert_eq!(validate(SharedString::from(".")), SharedString::from("."));
        assert_eq!(validate(SharedString::from(" ")), SharedString::from(" "));
        assert_eq!(validate(SharedString::from("0")), SharedString::from("0"));
        assert_eq!(validate(SharedString::from("1")), SharedString::from("1"));
        assert_eq!(validate(SharedString::from("2")), SharedString::from("2"));
        assert_eq!(validate(SharedString::from("3")), SharedString::from("3"));
        assert_eq!(validate(SharedString::from("4")), SharedString::from("4"));
        assert_eq!(validate(SharedString::from("5")), SharedString::from("5"));
        assert_eq!(validate(SharedString::from("6")), SharedString::from("6"));
        assert_eq!(validate(SharedString::from("7")), SharedString::from("7"));
        assert_eq!(validate(SharedString::from("8")), SharedString::from("8"));
        assert_eq!(validate(SharedString::from("9")), SharedString::from("9"));
        assert_eq!(validate(SharedString::from("=")), SharedString::from(""));
        assert_eq!(
            validate(SharedString::from(Key::Shift)),
            SharedString::from("")
        );
        assert_eq!(
            validate(SharedString::from(Key::Menu)),
            SharedString::from("")
        );
        assert_eq!(
            validate(SharedString::from(Key::Control)),
            SharedString::from("")
        );
    }
}
