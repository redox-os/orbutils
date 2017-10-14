#![deny(warnings)]

extern crate html5ever;
extern crate orbclient;
extern crate orbfont;
extern crate orbimage;
extern crate orbtk;
extern crate url;
extern crate hyper;
extern crate hyper_rustls;


use std::{cmp, env, str};
use std::collections::BTreeMap;
use std::default::Default;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{stderr, Read, Write};
use std::string::String;
use std::time::Duration;

use html5ever::parse_document;
use html5ever::rcdom::{Document, Doctype, Text, Comment, Element, RcDom, Handle};
use html5ever::tendril::TendrilSink;
use orbclient::{Color, EventOption, Renderer, Window, WindowFlag, K_BKSP, K_ESC, K_LEFT, K_RIGHT, K_DOWN, K_PGDN, K_UP, K_PGUP, K_ENTER};
use orbfont::Font;
use url::Url;
use hyper::header::{self, Headers};
use hyper::Client;
use hyper::net::HttpsConnector;

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

fn text_block<'a>(string: &str, x: &mut i32, y: &mut i32, size: f32, bold: bool, color: Color, link: Option<String>, font: &'a Font, font_bold: &'a Font, window: &Window, blocks: &mut Vec<Block<'a>>) {
    let trimmed_left = string.trim_left();
    let left_margin = string.len() as i32 - trimmed_left.len() as i32;
    let trimmed_right = trimmed_left.trim_right();
    let right_margin = trimmed_left.len() as i32 - trimmed_right.len() as i32;

    //let escaped_text = escape_default(&trimmed_right);
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

        if *x + w >= window.width() as i32 && *x > 0 {
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

fn walk<'a>(handle: Handle, indent: usize, x: &mut i32, y: &mut i32, mut size: f32, mut bold: bool, mut color: Color, mut ignore: bool, whitespace: &mut bool, mut link: Option<String>, url: &Url, font: &'a Font, font_bold: &'a Font, window: &Window, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    let node = handle.borrow();

    let mut new_line = false;

    //print!("{}", repeat(" ").take(indent).collect::<String>());
    match node.node {
        Document
            => {
                //println!("#Document")
            },

        Doctype(ref _name, ref _public, ref _system)
            => {
                //println!("<!DOCTYPE {} \"{}\" \"{}\">", *name, *public, *system);
            },

        Text(ref text)
            => {
                let mut string = String::new();

                for c in text.chars() {
                    match c {
                        ' ' | '\t' | '\n' | '\r' => if *whitespace {
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
                        text_block(&string, x, y, size, bold, color, link.clone(), font, font_bold, window, blocks);
                    }
                } else {
                    //println!("#text: empty");
                }
            },

        Comment(ref _text)
            => {
                //println!("<!-- {} -->", escape_default(text))
            },

        Element(ref name, _, ref attrs) => {
            /*
            assert!(name.ns == ns!(html));
            //print!("<{}", name.local);
            for attr in attrs.iter() {
                assert!(attr.name.ns == ns!());
                //print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            //println!(">");
            */

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
                                if let Ok((_img_headers, img_data)) = http_download(&img_url) {
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
                                }
                            } else if src.ends_with(".png") {
                                let img_url = url.join(&src).unwrap();
                                if let Ok((_img_headers, img_data)) = http_download(&img_url) {
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
                        }

                        if use_alt {
                            if let Some(alt) = alt_opt {
                                text_block(&alt, x, y, size, bold, color, link.clone(), font, font_bold, window, blocks);
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
        walk(child.clone(), indent + 4, x, y, size, bold, color, ignore, whitespace, link.clone(), url, font, font_bold, window, anchors, blocks);
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

fn http_download(url: &Url) -> Result<(Headers, Vec<u8>), String> {
    write!(stderr(), "* Requesting {}\n", url).map_err(|err| format!("{}", err))?;

    let mut client = Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
    client.set_read_timeout(Some(Duration::new(5, 0)));
    client.set_write_timeout(Some(Duration::new(5, 0)));
    let mut res = client.get(url.clone()).send().map_err(|err| format!("Failed to send request: {}", err))?;
    let mut data = Vec::new();
    res.read_to_end(&mut data).map_err(|err| format!("Failed to read response: {}", err))?;

    write!(stderr(), "* Received {} bytes\n", data.len()).map_err(|err| format!("{}", err))?;

    Ok((res.headers.clone(), data))
}

fn read_parse<'a, R: Read>(headers: Headers, r: &mut R, url: &Url, font: &'a Font, font_bold: &'a Font, window: &Window, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    let content_type = headers.get_raw("content-type").and_then(|x| str::from_utf8(x[0].as_slice()).ok()).unwrap_or("text/plain");
    let media_type = content_type.split(";").next().unwrap_or("");

    match media_type {
        "text/plain" => {
            let mut string = String::new();
            match r.read_to_string(&mut string) {
                Ok(_) => {
                    let mut y = 0;
                    for line in string.lines() {
                        text_block(line, &mut 0, &mut y, 12.0, false, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                        y += 12;
                    }
                },
                Err(err) => {
                    let error = format!("Text data not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                }
            }
        },
        "text/html" => {
            match parse_document(RcDom::default(), Default::default()).from_utf8().read_from(r) {
                Ok(dom) => {
                    let mut x = 0;
                    let mut y = 0;
                    let mut whitespace = false;
                    walk(dom.document, 0, &mut x, &mut y, 16.0, false, Color::rgb(0, 0, 0), false, &mut whitespace, None, url, font, font_bold, window, anchors, blocks);

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
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                }
            }
        },
        "image/jpeg" => {
            let mut data = Vec::new();
            match r.read_to_end(&mut data) {
                Ok(_) => match orbimage::parse_jpg(&data) {
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
                        text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                    }
                },
                Err(err) => {
                    let error = format!("JPG stream not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                }
            }
        },
        "image/png" => {
            let mut data = Vec::new();
            match r.read_to_end(&mut data){
                Ok(_) => match orbimage::parse_png(&data) {
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
                        text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                    }
                },
                Err(err) => {
                    let error = format!("PNG stream not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                }
            }
        },
        "image/x-ms-bmp" => {
            let mut data = Vec::new();
            match r.read_to_end(&mut data) {
                Ok(_) => match orbimage::parse_bmp(&data) {
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
                        text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                    }
                },
                Err(err) => {
                    let error = format!("BMP stream not readable: {}", err);
                    text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
                }
            }
        },
        _ => {
            let error = format!("Unsupported content type: {}", content_type);
            text_block(&error, &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, window, blocks);
        }
    }
}

fn file_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, window: &Window, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    if let Ok(path) = url.to_file_path() {
        if let Ok(mut file) = File::open(&path) {
            let mut headers = Headers::new();

            let mime_type = match path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("") {
                "html" => "text/html",
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "bmp" => "image/x-ms-bmp",
                _ => "text/plain",
            };

            /* TODO {
                let extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                let mime_type = mime_guess::get_mime_type_str(extension).unwrap_or("application/octet-stream");
                println!("{:?}", mime_type);
            } */

            headers.set(header::ContentType(mime_type.parse().unwrap()));

            read_parse(headers, &mut file, url, font, font_bold, window, anchors, blocks);
        } else {
            println!("{} not found", path.display());
        }
    }
}

fn http_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, window: &Window, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    match http_download(url) {
        Ok((headers, response)) => {
            read_parse(headers, &mut response.as_slice(), url, font, font_bold, window, anchors, blocks);
        },
        Err(err) => {
            let mut headers = Headers::new();
            headers.set(header::ContentType("text/plain".parse().unwrap()));
            let response = format!("{}", err).into_bytes();
            read_parse(headers, &mut response.as_slice(), url, font, font_bold, window, anchors, blocks);
        }
    }
}

fn url_parse<'a>(url: &Url, font: &'a Font, font_bold: &'a Font, window: &Window, anchors: &mut BTreeMap<String, i32>, blocks: &mut Vec<Block<'a>>) {
    if url.scheme() == "http" || url.scheme() == "https" {
        http_parse(url, font, font_bold, window, anchors, blocks)
    } else if url.scheme() == "file" {
        file_parse(url, font, font_bold, window, anchors, blocks)
    } else {
        println!("{} scheme not found", url.scheme());
    }
}

fn open_dialog(url: &Url) -> Option<Url> {
    use orbtk::{Button, Click, Enter, Place, Point, Rect, Text, TextBox, Window};
    use std::cell::RefCell;
    use std::rc::Rc;

    let ret = Rc::new(RefCell::new(None));

    {
        let w = 400;
        let mut window = Window::new(Rect::new(-1, -1, w, 8 + 28 + 8 + 28 + 8), "Open");

        let path_box = TextBox::new();
        {
            let ret_path = ret.clone();
            let window_path = &mut window as *mut Window;
            path_box.position(8, 8)
                .size(w - 16, 28)
                .text_offset(6, 6)
                .text(format!("{}", url))
                .on_enter(move |me: &TextBox| {
                    if let Ok(new_url) = Url::parse(&me.text.get()) {
                        *ret_path.borrow_mut() = Some(new_url);
                    }
                    unsafe { (&mut *window_path).close(); }
                });
                window.add(&path_box);
        }

        {
            let window_cancel = &mut window as *mut Window;
            let button = Button::new();
            button.position(8, 8 + 28 + 8)
                .size((w - 16)/2 - 4, 28)
                .text_offset(6, 6)
                .text("Cancel")
                .on_click(move |_button: &Button, _point: Point| {
                    unsafe { (&mut *window_cancel).close(); }
                });
            window.add(&button);
        }

        {
            let ret_open = ret.clone();
            let window_open = &mut window as *mut Window;
            let button = Button::new();
            button.position((w as i32)/2 + 4, 8 + 28 + 8)
                .size((w - 16)/2 - 4, 28)
                .text_offset(6, 6)
                .text("Open")
                .on_click(move |_button: &Button, _point: Point| {
                    if let Ok(new_url) = Url::parse(&path_box.text.get()) {
                        *ret_open.borrow_mut() = Some(new_url);
                    }
                    unsafe { (&mut *window_open).close(); }
                });
            window.add(&button);
        }

        window.exec();
    }

    Rc::try_unwrap(ret).unwrap().into_inner()
}

fn main_window(arg: &str, font: &Font, font_bold: &Font) {
    let mut history = vec![];

    let mut url = Url::parse(arg).unwrap();

    let (display_width, display_height) = orbclient::get_display_size().expect("viewer: failed to get display size");
    let (window_w, window_h) = (cmp::min(1024, display_width * 4/5) as i32, cmp::min(768, display_height * 4/5) as i32);

    let mut window = Window::new_flags(
        -1, -1, window_w as u32, window_h as u32,  "Browser", &[WindowFlag::Resizable]
    ).unwrap();

    let mut anchors = BTreeMap::new();
    let mut blocks = Vec::new();

    let mut offset = (0, 0);
    let mut max_offset = (0, 0);

    let mut mouse_x = 0;
    let mut mouse_y = 0;
    let mut mouse_down = false;

    let mut reload = true;
    let mut redraw = true;
    loop {
        if reload {
            reload = false;

            window.set_title(&format!("{} - Browser", url));

            anchors.clear();
            blocks.clear();
            text_block("Loading...", &mut 0, &mut 0, 16.0, true, Color::rgb(0, 0, 0), None, font, font_bold, &window, &mut blocks);

            {
                window.set(Color::rgb(255, 255, 255));

                for block in blocks.iter() {
                    block.draw(&mut window, (0, 0));
                }

                window.sync();
            }

            anchors.clear();
            blocks.clear();
            url_parse(&url, &font, &font_bold, &window, &mut anchors, &mut blocks);

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
                EventOption::Key(key_event) => match key_event.scancode {
                    K_LEFT if key_event.pressed => {
                        redraw = true;
                        offset.0 = cmp::max(0, offset.0 - 60);
                    },
                    K_RIGHT if key_event.pressed => {
                        redraw = true;
                        offset.0 = cmp::min(cmp::max(0, max_offset.0 - window_w), offset.0 + 60);
                    },
                    K_UP if key_event.pressed => {
                        redraw = true;
                        offset.1 = cmp::max(0, offset.1 - 60);
                    },
                    K_PGUP if key_event.pressed => {
                        redraw = true;
                        offset.1 = cmp::max(0, offset.1 - 600);
                    },
                    K_DOWN if key_event.pressed => {
                        redraw = true;
                        offset.1 = cmp::min(cmp::max(0, max_offset.1 - window_h), offset.1 + 60);
                    },
                    K_PGDN if key_event.pressed => {
                        redraw = true;
                        offset.1 = cmp::min(cmp::max(0, max_offset.1 - window_h), offset.1 + 600);
                    },
                    K_BKSP if ! key_event.pressed => if let Some(last_url) = history.pop() {
                        url = last_url;
                        reload = true;
                    },
                    K_ENTER if ! key_event.pressed => {
                        if let Some(new_url) = open_dialog(&url) {
                            url = new_url;
                            reload = true;
                        }
                    },
                    _ => ()
                },
                EventOption::Mouse(mouse_event) => {
                    mouse_x = mouse_event.x;
                    mouse_y = mouse_event.y;
                },
                EventOption::Button(button_event) => {
                    if button_event.left {
                        mouse_down = true;
                    } else if mouse_down {
                        mouse_down = false;

                        let mut link_opt = None;
                        for block in blocks.iter() {
                            if block.contains(mouse_x, mouse_y, offset) {
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
                    }
                },
                EventOption::Scroll(scroll_event) => {
                    offset.0 = cmp::max(0, cmp::min(cmp::max(0, max_offset.0 - window_w), offset.0 - scroll_event.x * 48));
                    offset.1 = cmp::max(0, cmp::min(cmp::max(0, max_offset.1 - window_h), offset.1 - scroll_event.y * 48));

                    redraw = true;
                },
                EventOption::Resize(_) => {
                    redraw = true;
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
            Ok(font_bold) => main_window(&env::args().nth(1).unwrap_or("https://www.redox-os.org".to_string()), &font, &font_bold),
            Err(err) => err_window(&format!("{}", err))
        },
        Err(err) => err_window(&format!("{}", err))
    }
}
