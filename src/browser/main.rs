#[macro_use] extern crate html5ever_atoms;
extern crate html5ever;
extern crate orbclient;
extern crate orbfont;
extern crate tendril;

use std::env;
use std::iter::repeat;
use std::default::Default;
use std::string::String;

use html5ever::parse_document;
use html5ever::rcdom::{Document, Doctype, Text, Comment, Element, RcDom, Handle};
use orbclient::{Color, Window, EventOption, K_ESC};
use orbfont::Font;
use tendril::TendrilSink;

// This is not proper HTML serialization, of course.

struct Block<'a> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Color,
    text: orbfont::Text<'a>
}

impl<'a> Block<'a> {
    fn draw(&self, window: &mut Window) {
        if self.x < window.width() as i32 && self.y < window.height() as i32 {
            self.text.draw(window, self.x, self.y, self.color);
        }
    }
}

fn walk<'a>(handle: Handle, indent: usize, x: &mut i32, y: &mut i32, mut size: f32, mut bold: bool, mut color: Color, mut ignore: bool, whitespace: &mut bool, font: &'a Font, font_bold: &'a Font, blocks: &mut Vec<Block<'a>>) {
    let node = handle.borrow();

    let mut new_line = false;

    print!("{}", repeat(" ").take(indent).collect::<String>());
    match node.node {
        Document
            => {
                println!("#Document")
            },

        Doctype(ref name, ref public, ref system)
            => {
                println!("<!DOCTYPE {} \"{}\" \"{}\">", *name, *public, *system);
            },

        Text(ref text)
            => {
                let mut block_text = String::new();

                for c in text.chars() {
                    match c {
                        ' ' | '\n' | '\r' => if *whitespace {
                            // Ignore
                        } else {
                            // Set whitespace
                            *whitespace = true;
                            block_text.push(' ');
                        },
                        _ => {
                            if *whitespace {
                                *whitespace = false;
                            }
                            block_text.push(c);
                        }
                    }
                }

                if ! block_text.is_empty() {
                    if ignore {
                        println!("#text: ignored");
                    } else {
                        let trimmed_left = block_text.trim_left();
                        let left_margin = block_text.len() as i32 - trimmed_left.len() as i32;
                        let trimmed_right = trimmed_left.trim_right();
                        let right_margin = trimmed_left.len() as i32 - trimmed_right.len() as i32;

                        let escaped_text = escape_default(&trimmed_right);
                        println!("#text: block {} at {}, {}: '{}'", blocks.len(), *x, *y, escaped_text);

                        *x += left_margin * 8;

                        for (word_i, word) in trimmed_right.split(' ').enumerate() {
                            if word_i > 0 {
                                *x += 8;
                            }

                            let text = if bold {
                                font_bold.render(word, size)
                            } else {
                                font.render(word, size)
                            };

                            let w = text.width() as i32;
                            let h = text.height() as i32;

                            if *x + w >= 640 && *x > 0 {
                                *x = 0;
                                *y += size.ceil() as i32;
                            }

                            blocks.push(Block {
                                x: *x,
                                y: *y,
                                w: w,
                                h: h,
                                color: color,
                                text: text
                            });

                            *x += w;
                        }

                        *x += right_margin * 8;
                    }
                } else {
                    println!("#text: empty");
                }
            },

        Comment(ref text)
            => {
                println!("<!-- {} -->", escape_default(text))
            },

        Element(ref name, _, ref _attrs) => {
            assert!(name.ns == ns!(html));
            print!("<{}", name.local);
            /*
            for attr in attrs.iter() {
                assert!(attr.name.ns == ns!());
                print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            */
            println!(">");

            match &*name.local {
                "a" => {
                    color = Color::rgb(0, 0, 255);
                },
                "b" => {
                    bold = true;
                },
                "br" => {
                    ignore = true;
                    new_line = true;
                },
                "div" => {
                    new_line = true;
                },
                "h1" => {
                    size = 32.0;
                    bold = true;
                    new_line = true;
                },
                "h2" => {
                    size = 24.0;
                    bold = true;
                    new_line = true;
                },
                "h3" => {
                    size = 18.0;
                    bold = true;
                    new_line = true;
                }
                "h4" => {
                    size = 16.0;
                    bold = true;
                    new_line = true;
                }
                "h5" => {
                    size = 14.0;
                    bold = true;
                    new_line = true;
                }
                "h6" => {
                    size = 10.0;
                    bold = true;
                    new_line = true;
                },
                "li" => {
                    new_line = true;
                },
                "p" => {
                    new_line = true;
                },

                "head" => ignore = true,
                "title" => ignore = true, //TODO: Grab title
                "link" => ignore = true,
                "meta" => ignore = true,
                "script" => ignore = true,
                _ => ()
            }
        }
    }

    for child in node.children.iter() {
        walk(child.clone(), indent + 4, x, y, size, bold, color, ignore, whitespace, font, font_bold, blocks);
    }

    if new_line {
        *whitespace = true;
        *x = 0;
        *y += size.ceil() as i32;
    }
}

// FIXME: Copy of str::escape_default from std, which is currently unstable
pub fn escape_default(s: &str) -> String {
    s.chars().flat_map(|c| c.escape_default()).collect()
}

fn event_loop(window: &mut Window){
    loop {
        for event in window.events() {
            if let EventOption::Key(key_event) = event.to_option() {
                if key_event.pressed && key_event.scancode == K_ESC {
                    return;
                }
            }
            if let EventOption::Quit(_) = event.to_option() {
                return;
            }
        }
    }
}

fn error_msg(window: &mut Window, msg: &str) {
    let mut x = 0;
    for c in msg.chars() {
        window.char(x, 0, c, Color::rgb(255, 255, 255));
        x += 8;
    }
}

fn main() {
    let title = match env::args().nth(1) {
        Some(arg) => arg.clone(),
        None => "".to_string(),
    };

    match Font::find(None, None, None) {
        Ok(font) => match Font::find(None, None, Some("Bold")) {
            Ok(font_bold) => {
                let dom = parse_document(RcDom::default(), Default::default())
                    .from_utf8()
                    .from_file(env::args().nth(1).expect("browser: no file provided"))
                    .unwrap();

                let mut blocks = vec![];
                let mut x = 0;
                let mut y = 0;
                let mut whitespace = false;
                walk(dom.document, 0, &mut x, &mut y, 16.0, false, Color::rgb(0, 0, 0), false, &mut whitespace, &font, &font_bold, &mut blocks);

                if !dom.errors.is_empty() {
                    /*
                    println!("\nParse errors:");
                    for err in dom.errors.into_iter() {
                        println!("    {}", err);
                    }
                    */
                }

                let mut window = Window::new(-1,
                                             -1,
                                             640,
                                             480,
                                             &("Browser (".to_string() + &title + ")"))
                                     .unwrap();
                window.set(Color::rgb(255, 255, 255));
                let block_len = blocks.len();
                for (block_i, block) in blocks.iter().enumerate() {
                    println!("Draw block {}/{}", block_i, block_len);
                    block.draw(&mut window);
                }
                window.sync();
                event_loop(&mut window);
            },
            Err(err) => {
                let mut window = Window::new(-1,
                                             -1,
                                             320,
                                             32,
                                             &("Browser (".to_string() + &title + ")"))
                                     .unwrap();
                window.set(Color::rgb(0, 0, 0));
                error_msg(&mut window, &format!("{}", err));
                window.sync();
                event_loop(&mut window);
            }
        },
        Err(err) => {
            let mut window = Window::new(-1,
                                         -1,
                                         320,
                                         32,
                                         &("Browser (".to_string() + &title + ")"))
                                 .unwrap();
            window.set(Color::rgb(0, 0, 0));
            error_msg(&mut window, &format!("{}", err));
            window.sync();
            event_loop(&mut window);
        }
    }
}
