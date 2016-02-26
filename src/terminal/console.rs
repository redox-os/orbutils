use std::mem;

use orbclient::{Color, Event, EventOption, Window};
use orbclient::event;

const BLACK: Color = Color::rgb(0, 0, 0);
const RED: Color = Color::rgb(194, 54, 33);
const GREEN: Color = Color::rgb(37, 188, 36);
const YELLOW: Color = Color::rgb(173, 173, 39);
const BLUE: Color = Color::rgb(73, 46, 225);
const MAGENTA: Color = Color::rgb(211, 56, 211);
const CYAN: Color = Color::rgb(51, 187, 200);
const WHITE: Color = Color::rgb(203, 204, 205);

pub struct Console {
    pub window: Box<Window>,
    pub point_x: i32,
    pub point_y: i32,
    pub foreground: Color,
    pub background: Color,
    pub redraw: bool,
    pub command: String,
    pub escape: bool,
    pub escape_sequence: bool,
    pub sequence: Vec<String>,
}

impl Console {
    pub fn new() -> Console {
        let mut window = Window::new(-1, -1, 640, 480, "Terminal").unwrap();
        window.sync();
        Console {
            window: window,
            point_x: 0,
            point_y: 0,
            foreground: WHITE,
            background: BLACK,
            redraw: true,
            command: String::new(),
            escape: false,
            escape_sequence: false,
            sequence: Vec::new(),
        }
    }

    pub fn code(&mut self, c: char) {
        if self.escape_sequence {
            if c >= '0' && c <= '9' {
                // Add a number to the sequence list
                if let Some(mut value) = self.sequence.last_mut() {
                    value.push(c);
                }
            } else if c == ';' {
                // Split sequence into list
                self.sequence.push(String::new());
            } else if c == 'm' {
                // Display attributes
                for value in self.sequence.iter() {
                    if value == "0" {
                        // Reset all
                        self.foreground = WHITE;
                        self.background = BLACK;
                    } else if value == "30" {
                        self.foreground = BLACK;
                    } else if value == "31" {
                        self.foreground = RED;
                    } else if value == "32" {
                        self.foreground = GREEN;
                    } else if value == "33" {
                        self.foreground = YELLOW;
                    } else if value == "34" {
                        self.foreground = BLUE;
                    } else if value == "35" {
                        self.foreground = MAGENTA;
                    } else if value == "36" {
                        self.foreground = CYAN;
                    } else if value == "37" {
                        self.foreground = WHITE;
                    } else if value == "40" {
                        self.background = BLACK;
                    } else if value == "41" {
                        self.background = RED;
                    } else if value == "42" {
                        self.background = GREEN;
                    } else if value == "43" {
                        self.background = YELLOW;
                    } else if value == "44" {
                        self.background = BLUE;
                    } else if value == "45" {
                        self.background = MAGENTA;
                    } else if value == "46" {
                        self.background = CYAN;
                    } else if value == "47" {
                        self.background = WHITE;
                    }
                }

                self.escape_sequence = false;
            } else {
                self.escape_sequence = false;
            }

            if !self.escape_sequence {
                self.sequence.clear();
                self.escape = false;
            }
        } else if c == '[' {
            // Control sequence initiator

            self.escape_sequence = true;
            self.sequence.push(String::new());
        } else if c == 'c' {
            // Reset
            self.point_x = 0;
            self.point_y = 0;
            self.foreground = WHITE;
            self.background = BLACK;
            self.window.set(self.background);
            self.redraw = true;

            self.escape = false;
        } else {
            // Unknown escape character

            self.escape = false;
        }
    }

    pub fn scroll(&mut self, rows: usize) {
        if rows > 0 && rows < self.window.height() as usize {
            let offset = rows * self.window.width() as usize;
            let data = self.window.data_mut();
            for i in 0..data.len() - offset {
                let color = data[i + offset];
                data[i] = color;
            }
            for i in data.len() - offset..data.len() {
                data[i] = self.background;
            }
        }
    }

    pub fn character(&mut self, c: char) {
        self.window.rect(self.point_x, self.point_y, 8, 16, self.background);
        if c == '\x00' {
            // Ignore null character
        } else if c == '\x1B' {
            self.escape = true;
        } else if c == '\n' {
            self.point_x = 0;
            self.point_y += 16;
        } else if c == '\t' {
            self.point_x = ((self.point_x / 64) + 1) * 64;
        } else if c == '\x08' {
            self.point_x -= 8;
            if self.point_x < 0 {
                self.point_x = 0
            }
            self.window.rect(self.point_x, self.point_y, 8, 16, self.background);
        } else {
            self.window.char(self.point_x, self.point_y, c, self.foreground);
            self.point_x += 8;
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
                    match key_event.scancode {
                        event::K_BKSP => if ! self.command.is_empty() {
                            self.write(&[8]);
                            self.command.pop();
                        },
                        _ => match key_event.character {
                            '\0' => (),
                            c => {
                                self.command.push(c);
                                self.write(&[c as u8]);

                                if c == '\n' {
                                    let mut command = String::new();
                                    mem::swap(&mut self.command, &mut command);
                                    return Some(command);
                                }
                            }
                        },
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
