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
    pub raw_mode: bool,
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
                    for value in self.sequence.iter() {
                        match value.as_str() {
                            "0" => {
                                self.foreground = WHITE;
                                self.background = BLACK;
                            },
                            "30" => self.foreground = BLACK,
                            "31" => self.foreground = RED,
                            "32" => self.foreground = GREEN,
                            "33" => self.foreground = YELLOW,
                            "34" => self.foreground = BLUE,
                            "35" => self.foreground = MAGENTA,
                            "36" => self.foreground = CYAN,
                            "37" => self.foreground = WHITE,
                            "40" => self.background = BLACK,
                            "41" => self.background = RED,
                            "42" => self.background = GREEN,
                            "43" => self.background = YELLOW,
                            "44" => self.background = BLUE,
                            "45" => self.background = MAGENTA,
                            "46" => self.background = CYAN,
                            "47" => self.background = WHITE,
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

                    let row = self.sequence.get(0).map_or("", |p| &p).parse::<i32>().unwrap_or(0);
                    self.point_y = row * 16;

                    let col = self.sequence.get(1).map_or("", |p| &p).parse::<i32>().unwrap_or(0);
                    self.point_x = col * 8;

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
                'r' => self.raw_mode = true,
                'R' => self.raw_mode = false,
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
                    self.foreground = WHITE;
                    self.background = BLACK;
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
                self.window.char(self.point_x, self.point_y, c, self.foreground);
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
                            event::K_BKSP => self.command.push_str("\x08 \x08"),
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
