extern crate system;

extern crate orbfont;

use self::system::graphics::{fast_copy, fast_set};

use self::orbfont::Font;

use std::{cmp, mem};

use orbclient::{Color, Event, EventOption, Window};
use orbclient::event;

fn ansi_color(value: u8) -> Color {
    match value {
        0 => Color::rgb(0x00, 0x00, 0x00),
        1 => Color::rgb(0x80, 0x00, 0x00),
        2 => Color::rgb(0x00, 0x80, 0x00),
        3 => Color::rgb(0x80, 0x80, 0x00),
        4 => Color::rgb(0x00, 0x00, 0x80),
        5 => Color::rgb(0x80, 0x00, 0x80),
        6 => Color::rgb(0x00, 0x80, 0x80),
        7 => Color::rgb(0xc0, 0xc0, 0xc0),
        8 => Color::rgb(0x80, 0x80, 0x80),
        9 => Color::rgb(0xff, 0x00, 0x00),
        10 => Color::rgb(0x00, 0xff, 0x00),
        11 => Color::rgb(0xff, 0xff, 0x00),
        12 => Color::rgb(0x00, 0x00, 0xff),
        13 => Color::rgb(0xff, 0x00, 0xff),
        14 => Color::rgb(0x00, 0xff, 0xff),
        15 => Color::rgb(0xff, 0xff, 0xff),
        16 ... 231 => {
            let convert = |value: u8| -> u8 {
                match value {
                    0 => 0,
                    _ => value * 0x28 + 0x28
                }
            };

            let r = convert((value - 16)/36 % 6);
            let g = convert((value - 16)/6 % 6);
            let b = convert((value - 16) % 6);
            Color::rgb(r, g, b)
        },
        232 ... 255 => {
            let gray = (value - 232) * 10 + 8;
            Color::rgb(gray, gray, gray)
        },
        _ => Color::rgb(0, 0, 0)
    }
}

pub struct Console {
    pub window: Window,
    pub font: Font,
    pub font_bold: Font,
    pub point_x: i32,
    pub point_y: i32,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub redraw: bool,
    pub command: String,
    pub escape: bool,
    pub escape_sequence: bool,
    pub sequence: Vec<String>,
    pub raw_mode: bool,
}

impl Console {
    pub fn new(width: u32, height: u32) -> Console {
        let mut window = Window::new_flags(-1, -1, width, height, "Terminal", true).unwrap();
        window.sync();
        Console {
            window: window,
            font: Font::find(Some("Mono"), None, Some("Regular")).unwrap(),
            font_bold: Font::find(Some("Mono"), None, Some("Bold")).unwrap(),
            point_x: 0,
            point_y: 0,
            foreground: ansi_color(7),
            background: ansi_color(0),
            bold: false,
            redraw: true,
            command: String::new(),
            escape: false,
            escape_sequence: false,
            sequence: Vec::new(),
            raw_mode: false,
        }
    }

    pub fn code(&mut self, c: char) {
        if self.escape_sequence {
            match c {
                '0' ... '9' => {
                    // Add a number to the sequence list
                    if let Some(mut value) = self.sequence.last_mut() {
                        value.push(c);
                    }
                },
                ';' => {
                    // Split sequence into list
                    self.sequence.push(String::new());
                },
                'm' => {
                    // Display attributes
                    let mut value_iter = self.sequence.iter();
                    while let Some(value_str) = value_iter.next() {
                        let value = value_str.parse::<u8>().unwrap_or(0);
                        match value {
                            0 => {
                                self.foreground = ansi_color(7);
                                self.background = ansi_color(0);
                                self.bold = false;
                            },
                            1 => {
                                self.bold = true;
                            },
                            30 ... 37 => self.foreground = ansi_color(value - 30),
                            38 => match value_iter.next().map_or("", |s| &s).parse::<usize>().unwrap_or(0) {
                                2 => {
                                    //True color
                                    let r = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let g = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let b = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.foreground = Color::rgb(r, g, b);
                                },
                                5 => {
                                    //256 color
                                    let color_value = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.foreground = ansi_color(color_value);
                                },
                                _ => {}
                            },
                            40 ... 47 => self.background = ansi_color(value - 40),
                            48 => match value_iter.next().map_or("", |s| &s).parse::<usize>().unwrap_or(0) {
                                2 => {
                                    //True color
                                    let r = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let g = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    let b = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.background = Color::rgb(r, g, b);
                                },
                                5 => {
                                    //256 color
                                    let color_value = value_iter.next().map_or("", |s| &s).parse::<u8>().unwrap_or(0);
                                    self.background = ansi_color(color_value);
                                },
                                _ => {}
                            },
                            _ => {},
                        }
                    }

                    self.escape_sequence = false;
                },
                'J' => {
                    match self.sequence.get(0).map_or("", |p| &p).parse::<usize>().unwrap_or(0) {
                        0 => {
                            //TODO: Erase down
                        },
                        1 => {
                            //TODO: Erase up
                        },
                        2 => {
                            // Erase all
                            self.point_x = 0;
                            self.point_y = 0;
                            self.window.set(self.background);
                            self.redraw = true;
                        },
                        _ => {}
                    }

                    self.escape_sequence = false;
                },
                'H' | 'f' => {
                    self.window.rect(self.point_x, self.point_y, 8, 16, self.background);

                    let row = self.sequence.get(0).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.point_y = cmp::max(0, row - 1) as i32 * 16;

                    let col = self.sequence.get(1).map_or("", |p| &p).parse::<isize>().unwrap_or(1);
                    self.point_x = cmp::max(0, col - 1) as i32 * 8;

                    self.window.rect(self.point_x, self.point_y, 8, 16, self.foreground);
                    self.redraw = true;

                    self.escape_sequence = false;
                },
/*
@MANSTART{terminal-raw-mode}
INTRODUCTION
    Since Redox has no ioctl syscall, it uses escape codes for switching to raw mode.

ENTERING AND EXITING RAW MODE
    Entering raw mode is done using CSI-r (^[r). Unsetting raw mode is done by CSI-R (^[R).

RAW MODE
    Raw mode means that the stdin must be handled solely by the program itself. It will not automatically be printed nor will it be modified in any way (modulo escape codes).

    This means that:
        - stdin is not printed.
        - newlines are interpreted as carriage returns in stdin.
        - stdin is not buffered, meaning that the stream of bytes goes directly to the program, without the user having to press enter.
@MANEND
*/
                'r' => {
                    self.raw_mode = true;
                    self.escape_sequence = false;
                },
                'R' => {
                    self.raw_mode = false;
                    self.escape_sequence = false;
                },
                _ => self.escape_sequence = false,

            }

            if !self.escape_sequence {
                self.sequence.clear();
                self.escape = false;
            }
        } else {
            match c {
                '[' => {
                    // Control sequence initiator

                    self.escape_sequence = true;
                    self.sequence.push(String::new());
                },
                'c' => {
                    // Reset
                    self.point_x = 0;
                    self.point_y = 0;
                    self.raw_mode = false;
                    self.foreground = ansi_color(7);
                    self.background = ansi_color(0);
                    self.bold = false;
                    self.window.set(self.background);
                    self.redraw = true;

                    self.escape = false;
                }
                _ => self.escape = false,
            }
        }
    }

    pub fn scroll(&mut self, rows: usize) {
        if rows > 0 && rows < self.window.height() as usize {
            let offset = rows * self.window.width() as usize;
            let data = self.window.data_mut();
            unsafe {
                fast_copy(data.as_mut_ptr() as *mut u32, data.as_ptr().offset(offset as isize) as *const u32, data.len() - offset);
                fast_set(data.as_mut_ptr().offset((data.len() - offset) as isize) as *mut u32, self.background.data, offset);
            }
        }
    }

    pub fn character(&mut self, c: char) {
        self.window.rect(self.point_x, self.point_y, 8, 16, self.background);

        match c {
            '\0' => {},
            '\x1B' => self.escape = true,
            '\n' => {
                self.point_x = 0;
                self.point_y += 16;
            },
            '\t' => self.point_x = ((self.point_x / 64) + 1) * 64,
            '\r' => self.point_x = 0,
            '\x08' => {
                self.point_x -= 8;
                if self.point_x < 0 {
                    self.point_x = 0
                }
                self.window.rect(self.point_x, self.point_y, 8, 16, self.background);
            },
            _ => {
                if self.bold {
                    self.font_bold.render(&c.to_string(), 16.0).draw(&mut self.window, self.point_x, self.point_y, self.foreground);
                } else {
                    self.font.render(&c.to_string(), 16.0).draw(&mut self.window, self.point_x, self.point_y, self.foreground);
                }
                //self.window.char(self.point_x, self.point_y, c, self.foreground);
                self.point_x += 8;
            }
        }

        if self.point_x >= self.window.width() as i32 {
            self.point_x = 0;
            self.point_y += 16;
        }

        while self.point_y + 16 > self.window.height() as i32 {
            self.scroll(16);
            self.point_y -= 16;
        }
        self.window.rect(self.point_x, self.point_y, 8, 16, self.foreground);
        self.redraw = true;
    }

    pub fn event(&mut self, event: Event) -> Option<String> {
        match event.to_option() {
            EventOption::Key(key_event) => {
                if key_event.pressed {
                    if self.raw_mode {
                        match key_event.scancode {
                            event::K_BKSP => self.command.push_str("\x08"),
                            event::K_UP => self.command.push_str("\x1B[A"),
                            event::K_DOWN => self.command.push_str("\x1B[B"),
                            event::K_RIGHT => self.command.push_str("\x1B[C"),
                            event::K_LEFT => self.command.push_str("\x1B[D"),
                            _ => match key_event.character {
                                '\0' => {},
                                c => {
                                    self.command.push(c);
                                }
                            },
                        }

                        if ! self.command.is_empty() {
                            let mut command = String::new();
                            mem::swap(&mut self.command, &mut command);
                            return Some(command);
                        }
                    } else {
                        match key_event.scancode {
                            event::K_BKSP => if ! self.command.is_empty() {
                                self.write(&[8]);
                                self.command.pop();
                            },
                            _ => match key_event.character {
                                '\0' => (),
                                c => {
                                    self.write(&[c as u8]);
                                    self.command.push(c);

                                    if c == '\n' {
                                        let mut command = String::new();
                                        mem::swap(&mut self.command, &mut command);
                                        return Some(command);
                                    }
                                }
                            },
                        }
                    }
                }
            },
            _ => (),
        }

        None
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            let c = *byte as char;

            if self.escape {
                self.code(c);
            } else {
                self.character(c);
            }
        }

        if self.redraw {
            self.redraw = false;
            self.window.sync();
        }
    }
}
