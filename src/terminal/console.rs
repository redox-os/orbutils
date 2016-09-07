extern crate orbfont;
extern crate ransid;

use self::orbfont::Font;

use std::mem;

use orbclient::{Color, Event, EventOption, Window};
use orbclient::event;

pub struct Console {
    pub window: Window,
    pub inner: ransid::Console,
    pub font: Font,
    pub font_bold: Font,
    pub command: String,
}

impl Console {
    pub fn new(width: u32, height: u32) -> Console {
        let mut window = Window::new_flags(-1, -1, width, height, "Terminal", true).unwrap();
        window.sync();
        Console {
            window: window,
            inner: ransid::Console::new(width as usize/8, height as usize/16),
            font: Font::find(Some("Mono"), None, None).unwrap(),
            font_bold: Font::find(Some("Mono"), None, Some("Bold")).unwrap(),
            command: String::new(),
        }
    }

    pub fn event(&mut self, event: Event) -> Option<String> {
        match event.to_option() {
            EventOption::Key(key_event) => {
                if key_event.pressed {
                    if self.inner.raw_mode {
                        match key_event.scancode {
                            event::K_BKSP => self.command.push_str("\x7F"),
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
                                self.inner.redraw = true;

                                self.write(&[8]);
                                self.command.pop();
                            },
                            _ => match key_event.character {
                                '\0' => (),
                                c => {
                                    self.inner.redraw = true;

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
        self.inner.write(bytes);
        if self.inner.redraw {
            self.inner.redraw = false;

            for y in 0..self.inner.h {
                if self.inner.changed[y] {
                    self.inner.changed[y] = false;

                    self.window.rect(0, y as i32 * 16, self.inner.h as u32 * 8, 16, Color {
                        data: self.inner.background.data
                    });
                    for x in 0..self.inner.w {
                        let block = self.inner.display[y * self.inner.w + x];
                        let (bg, fg) = if self.inner.cursor && self.inner.y == y && self.inner.x == x {
                            (block.fg.data, block.bg.data)
                        }else{
                            (block.bg.data, block.fg.data)
                        };
                        self.window.rect(x as i32 * 8, y as i32 * 16, 8, 16, Color {
                            data: bg
                        });
                        if block.c != ' ' {
                            if block.bold {
                                self.font_bold.render(&block.c.to_string(), 16.0).draw(&mut self.window, x as i32 * 8, y as i32 * 16, Color {
                                    data: fg
                                });
                            } else {
                                self.font.render(&block.c.to_string(), 16.0).draw(&mut self.window, x as i32 * 8, y as i32 * 16, Color {
                                    data: fg
                                });
                            }
                        }
                        if block.underlined {
                            self.window.rect(x as i32 * 8, y as i32 * 16 + 14, 8, 1, Color {
                                data: fg
                            });
                        }
                    }
                }
            }
            self.window.sync();
        }
    }
}
