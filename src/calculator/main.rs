#![deny(warnings)]

extern crate orbtk;

use orbtk::{Button, Placeable, Point, Rect, TextBox, Window};
use orbtk::callback::{Click, Enter};

#[derive(Debug, Clone)]
pub enum Token {
    Plus,
    Minus,
    Divide,
    Multiply,
    Exponent,
    OpenParen,
    CloseParen,
    Number(String),
}

impl Token {
    pub fn to_str(&self) -> &'static str {
        match self {
            &Token::Plus       => "Plus",
            &Token::Minus      => "Minus",
            &Token::Divide     => "Divide",
            &Token::Multiply   => "Multiply",
            &Token::Exponent   => "Exponent",
            &Token::OpenParen  => "OpenParen",
            &Token::CloseParen => "CloseParen",
            &Token::Number(_)  => "Number",
        }
    }

    pub fn to_string(&self) -> String {
        self.to_str().to_owned()
    }
}

#[derive(Debug,  Clone)]
pub enum ParseError {
    InvalidNumber(String),
    UnrecognizedToken(String),
    UnexpectedToken(String, &'static str),
    UnexpectedEndOfInput,
    OtherError(String),
}

#[derive(Clone,Debug)]
pub struct IntermediateResult {
    value: f64,
    tokens_read: usize,
}

impl IntermediateResult {
    fn new(value: f64, tokens_read: usize) -> Self {
        IntermediateResult {
            value: value,
            tokens_read: tokens_read,
        }
    }
}

pub trait OperatorFunctions {
    fn is_operator(self) -> bool;
    fn operator_type(self) -> Token;
}

impl OperatorFunctions for char {
    fn is_operator(self) -> bool {
        self == '+' ||
        self == '-' ||
        self == '*' ||
        self == '/' ||
        self == '^' ||
        self == '(' ||
        self == ')'
    }

    fn operator_type(self) -> Token {
        match self {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '/' => Token::Divide,
            '*' => Token::Multiply,
            '^' => Token::Exponent,
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            _   => panic!("Invalid operator")
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    // CONSIDER: input.len() too generous?
    let mut tokens = Vec::with_capacity(input.len());

    // TODO: Not this. Modify to use iterator
    let chars: Vec<char> = input.chars().collect();

    let input_length = chars.len();
    let mut current_pos = 0;
    while current_pos < input_length {
        let c = chars[current_pos];
        if c.is_digit(10) {
            let token_string = consume_number(&chars[current_pos..]);
            current_pos += token_string.len();
            tokens.push(Token::Number(token_string));
        } else if c.is_operator() {
            tokens.push(c.operator_type());
            current_pos += 1;
        } else if c.is_whitespace() {
            current_pos += 1;
        } else {
            let token_string = consume_until_new_token(&chars[current_pos..]);
            return Err(ParseError::UnrecognizedToken(token_string));
        }
    }
    Ok(tokens)
}

fn consume_number(input: &[char]) -> String {
    // CONSIDER: input.len seems a bit generous
    let mut number = String::with_capacity(input.len());
    let mut has_decimal_point = false;
    for &c in input {
        if c == '.' {
            if has_decimal_point {
                break;
            } else {
                number.push(c);
                has_decimal_point = true;
            }
        } else if c.is_digit(10) {
            number.push(c);
        } else {
            break;
        }
    }
    number
}

fn consume_until_new_token(input: &[char]) -> String {
    input.iter()
         .take_while(|c| !(c.is_whitespace() || c.is_operator() || c.is_digit(10)))
         .map(|&c| c)
         .collect()
}

// Addition and subtraction
pub fn e_expr(token_list: &[Token]) -> Result<IntermediateResult, ParseError> {
    let mut t1 = try!(t_expr(token_list));
    let mut index = t1.tokens_read;

    while index < token_list.len() {
        match token_list[index] {
            Token::Plus => {
                let t2 = try!(t_expr(&token_list[index+1..]));
                t1.value += t2.value;
                t1.tokens_read += t2.tokens_read + 1;
            }
            Token::Minus => {
                let t2 = try!(t_expr(&token_list[index+1..]));
                t1.value -= t2.value;
                t1.tokens_read += t2.tokens_read + 1;
            }
            Token::Number(ref n) => return Err(ParseError::UnexpectedToken(n.clone(),"operator")),
            _ => break,
        };
        index = t1.tokens_read;
    }
    Ok(t1)
}

// Multiplication and division
pub fn t_expr(token_list: &[Token]) -> Result<IntermediateResult, ParseError> {
    let mut f1 = try!(f_expr(token_list));
    let mut index = f1.tokens_read;

    while index < token_list.len() {
        match token_list[index] {
            Token::Multiply => {
                let f2 = try!(f_expr(&token_list[index+1..]));
                f1.value *= f2.value;
                f1.tokens_read += f2.tokens_read + 1;
            }
            Token::Divide => {
                let f2 = try!(f_expr(&token_list[index+1..]));
                if f2.value == 0.0 {
                    return Err(ParseError::OtherError("Divide by zero error".to_owned()));
                } else {
                    f1.value /= f2.value;
                    f1.tokens_read += f2.tokens_read + 1;
                }
            }
            Token::Number(ref n) => return Err(ParseError::UnexpectedToken(n.clone(),"operator")),
            _ => break,
        }
        index = f1.tokens_read;
    }
    Ok(f1)
}

// Exponentiation
pub fn f_expr(token_list: &[Token]) -> Result<IntermediateResult, ParseError> {
    let mut g1 = try!(g_expr(token_list));
    let mut index = g1.tokens_read;
    let token_len = token_list.len();
    while index < token_len {
        match token_list[index] {
            Token::Exponent => {
                let f = try!(f_expr(&token_list[index+1..]));
                g1.value = g1.value.powf(f.value);
                g1.tokens_read += f.tokens_read + 1;
            }
            Token::Number(ref n) => return Err(ParseError::UnexpectedToken(n.clone(),"operator")),
            _ => break,
        }
        index = g1.tokens_read;
    }
    Ok(g1)
}

// Numbers and parenthesized expressions
pub fn g_expr(token_list: &[Token]) -> Result<IntermediateResult, ParseError> {
    if !token_list.is_empty() {
        match token_list[0] {
            Token::Number(ref n) => {
                n.parse::<f64>()
                 .map_err(|_| ParseError::InvalidNumber(n.clone()))
                 .and_then(|num| Ok(IntermediateResult::new(num, 1)))
            }
            Token::Minus => {
                if token_list.len() > 1 {
                    if let Token::Number(ref n) = token_list[1] {
                        n.parse::<f64>()
                         .map_err(|_| ParseError::InvalidNumber(n.clone()))
                         .and_then(|num| Ok(IntermediateResult::new(-1.0 * num, 2)))
                    } else {
                        Err(ParseError::UnexpectedToken(token_list[1].to_string(), "number"))
                    }
                } else {
                    Err(ParseError::UnexpectedEndOfInput)
                }
            }
            Token::OpenParen => {
                let expr = e_expr(&token_list[1..]);
                match expr {
                    Ok(ir) => {
                        let close_paren = ir.tokens_read + 1;
                        if close_paren < token_list.len() {
                            match token_list[close_paren] {
                                Token::CloseParen => Ok(IntermediateResult::new(ir.value, close_paren+1)),
                                _ => Err(ParseError::UnexpectedToken(token_list[close_paren].to_string(), ")")),
                            }
                        } else {
                            Err(ParseError::OtherError("no matching close parenthesis found.".to_owned()))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            _ => Err(ParseError::UnexpectedToken(token_list[0].to_string(), "number"))
        }
    } else {
        Err(ParseError::UnexpectedEndOfInput)
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<String, ParseError> {
    e_expr(&tokens).map(|answer| answer.value.to_string())
}

fn eval(input: &str) -> String {
    match tokenize(input).and_then(parse) {
        Ok(s) => s,
        Err(_e) => "Syntax Error".to_string()
    }
}

fn main(){
    let window = Window::new(Rect::new(100, 100, 148, 200), "Calculator");

    {
        let text_box = TextBox::new()
            .position(0, 0)
            .size(300, 16)
            .on_enter(move |text_box: &TextBox| {
                let input = text_box.text.get();
                if ! input.is_empty() {
                    let result = eval(&input);
                    text_box.text_i.set(result.len());
                    text_box.text.set(result);
                }
            })
            .place(&window);

        let mut col = 0;
        let mut row = 0;

        {
            let mut btn = |name| {
                let text_box_clone = text_box.clone();
                Button::new()
                    .position(col * 36 + 4, row * 36 + 20)
                    .size(32, 32)
                    .text(name)
                    .text_offset(12, 8)
                    .on_click(move |_button: &Button, _point: Point| {
                        let text_i = text_box_clone.text_i.get();

                        let text = text_box_clone.text.get();

                        let mut new_text = String::new();
                        new_text.push_str(&text[.. text_i]);
                        new_text.push_str(name);
                        new_text.push_str(&text[text_i ..]);

                        text_box_clone.text.set(new_text);

                        text_box_clone.text_i.set(text_i + name.len());
                    })
                    .place(&window);
                col += 1;
                if col >= 4 {
                    col = 0;
                    row += 1;
                }
            };

            btn("("); btn(")"); btn("^"); btn("/");
            btn("7"); btn("8"); btn("9"); btn("*");
            btn("4"); btn("5"); btn("6"); btn("-");
            btn("1"); btn("2"); btn("3"); btn("+");
            btn("0"); btn("0"); btn(".");
        }

        {
            let text_box_clone = text_box.clone();
            Button::new()
                .position(col * 36 + 4, row * 36 + 20)
                .size(32, 32)
                .text("=")
                .text_offset(12, 8)
                .on_click(move |_button: &Button, _point: Point| {
                    text_box_clone.emit_enter();
                })
                .place(&window);
        }
    }

    window.exec();
}
