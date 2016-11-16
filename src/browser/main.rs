#[macro_use] extern crate html5ever_atoms;
extern crate html5ever;
//extern crate mime_guess;
extern crate orbclient;
extern crate orbfont;
extern crate orbimage;
extern crate tendril;
extern crate url;

use std::{cmp, env, str};
use std::collections::BTreeMap;
use std::iter::repeat;
use std::default::Default;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{stderr, Read, Write};
use std::net::TcpStream;
use std::string::String;

use html5ever::parse_document;
use html5ever::rcdom::{Document, Doctype, Text, Comment, Element, RcDom, Handle};
use orbclient::{Color, Window, EventOption, K_BKSP, K_ESC, K_LEFT, K_RIGHT, K_DOWN, K_PGDN, K_UP, K_PGUP};
use orbfont::Font;
use tendril::TendrilSink;
use url::Url;

struct Block<'a> {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Color,
    string: String,
    link: Option<String>,
    image: Option<orbimage::Image>,
    text: Option<orbfont::Text<'a>>,
}

impl<'a> Block<'a> {
    fn contains(&self, m_x: i32, m_y: i32, offset: (i32, i32)) -> bool {
        let x = self.x - offset.0;
        let y = self.y - offset.1;

        m_x >= x && m_x < x + self.w && m_y >= y && m_y < y + self.h
    }

    fn draw(&self, window: &mut Window, offset: (i32, i32)) {
        let x = self.x - offset.0;
        let y = self.y - offset.1;
        if x + self.w > 0 && x < window.width() as i32 && y + self.h > 0 && y < window.height() as i32 {
            if let Some(ref image) = self.image {
                image.draw(window, x, y);
            }

            if let Some(ref text) = self.text {
                text.draw(window, x, y, self.color);
            }
        }
    }
}

fn text_block<'a>(string: &str, x: &mut i32, y: &mut i32, size: f32, bold: bool, color: Color, link: Option<String>, font: &'a Font, font_bold: &'a Font, blocks: &mut Vec<Block<'a>>) {
    let trimmed_left = string.trim_left();
    let left_margin = string.len() as i32 - trimmed_left.len() as i32;
    let trimmed_right = trimmed_left.trim_right();
    let right_margin = trimmed_left.len() as i32 - trimmed_right.len() as i32;

    let escaped_text = escape_default(&trimmed_right);
    //println!("#text: block {} at {}, {}: '{}'", blocks.len(), *x, *y, escaped_text);

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

        if *x + w >= 800 && *x > 0 {
            *x = 0;
            *y += size.ceil() as i32;
        }

        blocks.push(Block {
            x: *x,
            y: *y,
            w: w,
            h: h,
            color: color,
            string: word.to_string(),
            link: link.clone(),
            image: None,
            text: Some(text)
        });

        *x += w;
    }

    *x += right_margin * 8;
}

fn walk<'a>(handle: Handle, indent: usize, x: &mut i32, y: &mut i32, mut size: f32, mut bold: bool, mut color: Color, mut ignore: bool, whitespace: &mut bool, mut link: Option<String>, url: &Url, font: &'a Font, font_bold: &'a Font, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    let node = handle.borrow();

    let mut new_line = false;

    //print!("{}", repeat(" ").take(indent).collect::<String>());
    match node.node {
        Document
            => {
                //println!("#Document")
            },

        Doctype(ref name, ref public, ref system)
            => {
                //println!("<!DOCTYPE {} \"{}\" \"{}\">", *name, *public, *system);
            },

        Text(ref text)
            => {
                let mut string = String::new();

                for c in text.chars() {
                    match c {
                        ' ' | '\n' | '\r' => if *whitespace {
                            // Ignore
                        } else {
                            // Set whitespace
                            *whitespace = true;
                            string.push(' ');
                        },
                        _ => {
                            if *whitespace {
                                *whitespace = false;
                            }
                            string.push(c);
                        }
                    }
                }

                if ! string.is_empty() {
                    if ignore {
                        //println!("#text: ignored");
                    } else {
                        text_block(&string, x, y, size, bold, color, link.clone(), font, font_bold, blocks);
                    }
                } else {
                    //println!("#text: empty");
                }
            },

        Comment(ref text)
            => {
                //println!("<!-- {} -->", escape_default(text))
            },

        Element(ref name, _, ref attrs) => {
            assert!(name.ns == ns!(html));
            //print!("<{}", name.local);
            for attr in attrs.iter() {
                assert!(attr.name.ns == ns!());
                //print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            //println!(">");

            match &*name.local {
                "a" => {
                    color = Color::rgb(0, 0, 255);
                    for attr in attrs.iter() {
                        match &*attr.name.local {
                            "name" => {
                                anchors.insert(attr.value.to_string(), *y);
                            },
                            "href" => {
                                link = Some(attr.value.to_string());
                            },
                            _ => ()
                        }
                    }
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
                "hr" => {
                    new_line = true;
                },
                "img" => {
                    if ! ignore {
                        let mut src_opt = None;
                        let mut alt_opt = None;
                        for attr in attrs.iter() {
                            match &*attr.name.local {
                                "src" => src_opt = Some(attr.value.to_string()),
                                "alt" => alt_opt = Some(attr.value.to_string()),
                                _ => ()
                            }
                        }

                        let mut use_alt = true;
                        if let Some(src) = src_opt {
                            if src.ends_with(".jpg") || src.ends_with(".jpeg") {
                                let img_url = url.join(&src).unwrap();
                                let (img_headers, img_data) = http_download(&img_url);
                                if let Ok(img) = orbimage::parse_jpg(&img_data) {
                                    use_alt = false;

                                    let w = img.width() as i32;
                                    let h = img.height() as i32;

                                    blocks.push(Block {
                                        x: *x,
                                        y: *y,
                                        w: w,
                                        h: h,
                                        color: color,
                                        string: String::new(),
                                        link: link.clone(),
                                        image: Some(img),
                                        text: None
                                    });

                                    *y += h;
                                }
                            } else if src.ends_with(".png") {
                                let img_url = url.join(&src).unwrap();
                                let (img_headers, img_data) = http_download(&img_url);
                                if let Ok(img) = orbimage::parse_png(&img_data) {
                                    use_alt = false;

                                    let w = img.width() as i32;
                                    let h = img.height() as i32;

                                    blocks.push(Block {
                                        x: *x,
                                        y: *y,
                                        w: w,
                                        h: h,
                                        color: color,
                                        string: String::new(),
                                        link: link.clone(),
                                        image: Some(img),
                                        text: None
                                    });

                                    *y += h;
                                }
                            }
                        }

                        if use_alt {
                            if let Some(alt) = alt_opt {
                                text_block(&alt, x, y, size, bold, color, link.clone(), font, font_bold, blocks);
                            }
                        }
                    }

                    ignore = true;
                    new_line = true;
                },
                "li" => {
                    new_line = true;
                },
                "p" => {
                    new_line = true;
                },
                "tr" => {
                    new_line = true;
                }

                "head" => ignore = true,
                "title" => ignore = true, //TODO: Grab title
                "link" => ignore = true,
                "meta" => ignore = true,
                "script" => ignore = true,
                "style" => ignore = true,
                _ => ()
            }
        }
    }

    for child in node.children.iter() {
        walk(child.clone(), indent + 4, x, y, size, bold, color, ignore, whitespace, link.clone(), url, font, font_bold, anchors, blocks);
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

fn http_download(url: &Url) -> (Vec<String>, Vec<u8>) {
    let host = url.host_str().unwrap_or("");
    let port = url.port().unwrap_or(80);
    let mut path = url.path().to_string();
    if let Some(query) = url.query() {
        path.push('?');
        path.push_str(query);
    }

    write!(stderr(), "* Connecting to {}:{}\n", host, port).unwrap();

    let mut stream = TcpStream::connect((host, port)).unwrap();

    write!(stderr(), "* Requesting {}\n", path).unwrap();

    let request = format!("GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
    stream.write(request.as_bytes()).unwrap();
    stream.flush().unwrap();

    write!(stderr(), "* Waiting for response\n").unwrap();

    let mut response = Vec::new();

    loop {
        let mut buf = [0; 65536];
        let count = stream.read(&mut buf).unwrap();
        if count == 0 {
            break;
        }
        response.extend_from_slice(&buf[.. count]);
    }

    write!(stderr(), "* Received {} bytes\n", response.len()).unwrap();

    let mut header_end = 0;
    while header_end < response.len() {
        if response[header_end..].starts_with(b"\r\n\r\n") {
            header_end += 4;
            break;
        }
        header_end += 1;
    }

    let mut headers = Vec::new();
    for line in unsafe { str::from_utf8_unchecked(&response[..header_end]) }.lines() {
        if ! line.is_empty() {
            write!(stderr(), "> {}\n", line).unwrap();
            headers.push(line.to_string());
        }
    }

    (headers, response.split_off(header_end))
}

fn read_parse<'a, R: Read>(headers: &Vec<String>, r: &mut R, url: &Url, font: &'a Font, font_bold: &'a Font, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    let mut content_type = "text/plain";
    for header in headers.iter() {
        if header.starts_with("Content-Type: ") {
            if let Some(new_type) = header[14..].split(';').next() {
                content_type = new_type;
                break;
            }
        }
    }

    match content_type {
        "text/plain" => {
            let mut string = String::new();
            match r.read_to_string(&mut string) {
                Ok(_) => {
                    let mut y = 0;
                    for line in string.lines() {
                        text_block(line, &mut 0, &mut y, 12.0, false, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                        y += 12;
                    }
                },
                Err(err) => {
                    let error = format!("Text data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                }
            }
        },
        "text/html" => {
            match parse_document(RcDom::default(), Default::default()).from_utf8().read_from(r) {
                Ok(dom) => {
                    let mut x = 0;
                    let mut y = 0;
                    let mut whitespace = false;
                    walk(dom.document, 0, &mut x, &mut y, 16.0, false, Color::rgb(0, 0, 0), false, &mut whitespace, None, url, font, font_bold, anchors, blocks);

                    if !dom.errors.is_empty() {
                        /*
                        println!("\nParse errors:");
                        for err in dom.errors.into_iter() {
                            println!("    {}", err);
                        }
                        */
                    }
                },
                Err(err) => {
                    let error = format!("HTML data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                }
            }
        },
        "image/jpeg" => {
            let mut data = Vec::new();
            r.read_to_end(&mut data).unwrap();
            match orbimage::parse_jpg(&data) {
                Ok(img) => {
                    blocks.push(Block {
                        x: 0,
                        y: 0,
                        w: img.width() as i32,
                        h: img.height() as i32,
                        color: Color::rgb(0, 0, 0),
                        string: String::new(),
                        link: None,
                        image: Some(img),
                        text: None
                    });
                },
                Err(err) => {
                    let error = format!("JPG data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                }
            }
        },
        "image/png" => {
            let mut data = Vec::new();
            r.read_to_end(&mut data).unwrap();
            match orbimage::parse_png(&data) {
                Ok(img) => {
                    blocks.push(Block {
                        x: 0,
                        y: 0,
                        w: img.width() as i32,
                        h: img.height() as i32,
                        color: Color::rgb(0, 0, 0),
                        string: String::new(),
                        link: None,
                        image: Some(img),
                        text: None
                    });
                },
                Err(err) => {
                    let error = format!("PNG data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                }
            }
        },
        "image/x-ms-bmp" => {
            let mut data = Vec::new();
            r.read_to_end(&mut data).unwrap();
            match orbimage::parse_bmp(&data) {
                Ok(img) => {
                    blocks.push(Block {
                        x: 0,
                        y: 0,
                        w: img.width() as i32,
                        h: img.height() as i32,
                        color: Color::rgb(0, 0, 0),
                        string: String::new(),
                        link: None,
                        image: Some(img),
                        text: None
                    });
                },
                Err(err) => {
                    let error = format!("BMP data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
                }
            }
        },
        _ => {
            let error = format!("Unsupported content type: {}", content_type);
            text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, blocks);
        }
    }
}

fn file_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    if let Ok(path) = url.to_file_path() {
        if let Ok(mut file) = File::open(&path) {
            let headers = Vec::new();

            /* TODO {
                let extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                let mime_type = mime_guess::get_mime_type_str(extension).unwrap_or("application/octet-stream");
                println!("{:?}", mime_type);
                headers.push(format!("Content-Type: {}", mime_type));
            } */

            read_parse(&headers, &mut file, url, &font, &font_bold, anchors, blocks);
        } else {
            println!("{} not found", path.display());
        }
    }
}

fn http_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    let (headers, response) = http_download(url);
    read_parse(&headers, &mut response.as_slice(), url, font, font_bold, anchors, blocks);
}

fn url_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    if url.scheme() == "http" || url.scheme() == "https" {
        http_parse(url, font, font_bold, anchors, blocks)
    } else if url.scheme() == "file" {
        file_parse(url, font, font_bold, anchors, blocks)
    } else {
        println!("{} scheme not found", url.scheme());
    }
}

fn main_window(arg: &str, font: &Font, font_bold: &Font) {
    let mut history = vec![];

    let mut url = Url::parse(arg).unwrap();

    let window_w = 800;
    let window_h = 600;
    let mut window = Window::new(-1, -1, window_w as u32, window_h as u32,  &format!("Browser ({})", arg)).unwrap();

    let mut anchors = BTreeMap::new();
    let mut blocks = Vec::new();

    let mut offset = (0, 0);
    let mut max_offset = (0, 0);

    let mut mouse_down = false;

    let mut reload = true;
    let mut redraw = true;
    loop {
        if reload {
            reload = false;

            anchors.clear();
            blocks.clear();
            url_parse(&url, &font, &font_bold, &mut anchors, &mut blocks);

            offset = (0, 0);
            max_offset = (0, 0);

            for block in blocks.iter() {
                if block.x + block.w > max_offset.0 {
                    max_offset.0 = block.x + block.w;
                }
                if block.y + block.h > max_offset.1 {
                    max_offset.1 = block.y + block.h;
                }
            }

            redraw = true;
        }

        if redraw {
            redraw = false;

            window.set(Color::rgb(255, 255, 255));

            for block in blocks.iter() {
                block.draw(&mut window, offset);
            }

            window.sync();
        }

        for event in window.events() {
            match event.to_option() {
                EventOption::Key(key_event) => if key_event.pressed {
                    match key_event.scancode {
                        K_ESC => return,
                        K_LEFT => {
                            redraw = true;
                            offset.0 = cmp::max(0, offset.0 - 60);
                        },
                        K_RIGHT => {
                            redraw = true;
                            offset.0 = cmp::min(cmp::max(0, max_offset.0 - window_w), offset.0 + 60);
                        },
                        K_UP => {
                            redraw = true;
                            offset.1 = cmp::max(0, offset.1 - 60);
                        },
                        K_PGUP => {
                            redraw = true;
                            offset.1 = cmp::max(0, offset.1 - 600);
                        },
                        K_DOWN => {
                            redraw = true;
                            offset.1 = cmp::min(cmp::max(0, max_offset.1 - window_h), offset.1 + 60);
                        },
                        K_PGDN => {
                            redraw = true;
                            offset.1 = cmp::min(cmp::max(0, max_offset.1 - window_h), offset.1 + 600);
                        },
                        K_BKSP => if let Some(last_url) = history.pop() {
                            url = last_url;
                            reload = true;
                        },
                        _ => ()
                    }
                },
                EventOption::Mouse(mouse_event) => if mouse_event.left_button {
                    mouse_down = true;
                } else if mouse_down {
                    mouse_down = false;

                    let mut link_opt = None;
                    for block in blocks.iter() {
                        if block.contains(mouse_event.x, mouse_event.y, offset) {
                            println!("Click {}", block.string);
                            if let Some(ref link) = block.link {
                                link_opt = Some(link.clone());
                                break;
                            }
                        }
                    }

                    if let Some(link) = link_opt {
                        if link.starts_with('#') {
                            if let Some(anchor) = anchors.get(&link[1..]) {
                                println!("Anchor {}: {}", link, *anchor);
                                offset.0 = 0;
                                offset.1 = *anchor;
                                redraw = true;
                            } else {
                                println!("Anchor {} not found", link);
                            }
                        } else {
                            history.push(url.clone());

                            url = url.join(&link).unwrap();

                            println!("Navigate {}: {:#?}", link, url);

                            reload = true;
                        }
                    }
                },
                EventOption::Quit(_) => return,
                _ => ()
            }
        }
    }
}

fn main() {
    let err_window = |msg: &str| {
        let mut window = Window::new(-1, -1, 320, 32, "Browser").unwrap();

        window.set(Color::rgb(0, 0, 0));

        let mut x = 0;
        for c in msg.chars() {
            window.char(x, 0, c, Color::rgb(255, 255, 255));
            x += 8;
        }

        window.sync();

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
    };

    match Font::find(None, None, None) {
        Ok(font) => match Font::find(None, None, Some("Bold")) {
            Ok(font_bold) => main_window(&env::args().nth(1).unwrap_or("http://www.redox-os.org".to_string()), &font, &font_bold),
            Err(err) => err_window(&format!("{}", err))
        },
        Err(err) => err_window(&format!("{}", err))
    }
}
