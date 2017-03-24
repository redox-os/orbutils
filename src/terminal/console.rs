extern crate ransid;

use std::cmp;
use std::collections::{BTreeSet, VecDeque};
use std::io::Result;

use orbclient::{Color, EventOption, Renderer, Window, WindowFlag};
use orbfont::Font;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_copy(dst: *mut u8, src: *const u8, len: usize) {
    asm!("cld
        rep movsb"
        :
        : "{rdi}"(dst as usize), "{rsi}"(src as usize), "{rcx}"(len)
        : "cc", "memory", "rdi", "rsi", "rcx"
        : "intel", "volatile");
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_set32(dst: *mut u32, src: u32, len: usize) {
    asm!("cld
        rep stosd"
        :
        : "{rdi}"(dst as usize), "{eax}"(src), "{rcx}"(len)
        : "cc", "memory", "rdi", "rcx"
        : "intel", "volatile");
}

#[derive(Clone, Copy)]
pub struct Block {
    c: char,
    fg: u32,
    bg: u32,
    bold: bool,
}

pub struct Console {
    pub console: ransid::Console,
    pub window: Window,
    pub grid: Box<[Block]>,
    pub font: Font,
    pub font_bold: Font,
    pub changed: BTreeSet<usize>,
    pub ctrl: bool,
    pub input: Vec<u8>,
    pub end_of_input: bool,
    pub cooked: VecDeque<u8>,
    pub requested: usize
}

impl Console {
    pub fn new(width: u32, height: u32) -> Console {
        let mut window = Window::new_flags(-1, -1, width, height, "Terminal", &[WindowFlag::Async, WindowFlag::Resizable]).unwrap();
        window.sync();

        let ransid = ransid::Console::new(width as usize / 8, height as usize / 16);
        let grid = vec![Block {
            c: '\0', fg: 0, bg: 0, bold: false
        }; ransid.w * ransid.h].into_boxed_slice();

        Console {
            console: ransid,
            grid: grid,
            window: window,
            font: Font::find(None, None, None).unwrap(),
            font_bold: Font::find(None, None, Some("Bold")).unwrap(),
            changed: BTreeSet::new(),
            ctrl: false,
            input: Vec::new(),
            end_of_input: false,
            cooked: VecDeque::new(),
            requested: 0
        }
    }

    pub fn input(&mut self, event_option: EventOption) {
        match event_option {
            EventOption::Key(key_event) => {
                let mut buf = vec![];

                if key_event.scancode == 0x1D {
                    self.ctrl = key_event.pressed;
                } else if key_event.pressed {
                    match key_event.scancode {
                        0x0E => { // Backspace
                            buf.extend_from_slice(b"\x7F");
                        },
                        0x1C => { // Enter
                            buf.extend_from_slice(b"\r");
                        }
                        0x47 => { // Home
                            buf.extend_from_slice(b"\x1B[H");
                        },
                        0x48 => { // Up
                            buf.extend_from_slice(b"\x1B[A");
                        },
                        0x49 => { // Page up
                            buf.extend_from_slice(b"\x1B[5~");
                        },
                        0x4B => { // Left
                            buf.extend_from_slice(b"\x1B[D");
                        },
                        0x4D => { // Right
                            buf.extend_from_slice(b"\x1B[C");
                        },
                        0x4F => { // End
                            buf.extend_from_slice(b"\x1B[F");
                        },
                        0x50 => { // Down
                            buf.extend_from_slice(b"\x1B[B");
                        },
                        0x51 => { // Page down
                            buf.extend_from_slice(b"\x1B[6~");
                        },
                        0x52 => { // Insert
                            buf.extend_from_slice(b"\x1B[2~");
                        },
                        0x53 => { // Delete
                            buf.extend_from_slice(b"\x1B[3~");
                        },
                        _ => {
                            let c = match key_event.character {
                                c @ 'A' ... 'Z' if self.ctrl => ((c as u8 - b'A') + b'\x01') as char,
                                c @ 'a' ... 'z' if self.ctrl => ((c as u8 - b'a') + b'\x01') as char,
                                c => c
                            };

                            if c != '\0' {
                                buf.extend_from_slice(&[c as u8]);
                            }
                        }
                    }
                }

                if self.console.raw_mode {
                    for &b in buf.iter() {
                        self.input.push(b);
                    }
                } else {
                    for &b in buf.iter() {
                        match b {
                            b'\x03' => {
                                self.end_of_input = true;
                                let _ = self.write(b"^C\n", true);
                            },
                            b'\x08' | b'\x7F' => {
                                if let Some(_c) = self.cooked.pop_back() {
                                    let _ = self.write(b"\x08", true);
                                }
                            },
                            b'\x1B' => {
                                let _ = self.write(b"^[", true);
                            },
                            b'\n' | b'\r' => {
                                self.cooked.push_back(b);
                                while let Some(c) = self.cooked.pop_front() {
                                    self.input.push(c);
                                }
                                let _ = self.write(b"\n", true);
                            },
                            _ => {
                                self.cooked.push_back(b);
                                let _ = self.write(&[b], true);
                            }
                        }
                    }
                }
            },
            EventOption::Mouse(_) => {
                //TODO: Mouse in terminal
            },
            EventOption::Resize(resize_event) => {
                let w = resize_event.width as usize/8;
                let h = resize_event.height as usize/16;

                let mut grid = vec![Block {
                    c: '\0', fg: self.console.foreground.data, bg: self.console.background.data, bold: false
                }; w * h].into_boxed_slice();

                self.window.set(Color { data: self.console.background.data });

                {
                    let font = &self.font;
                    let font_bold = &self.font_bold;
                    let window = &mut self.window;
                    let mut str_buf = [0; 4];
                    for y in 0..self.console.h {
                        for x in 0..self.console.w {
                            let block = self.grid[y * self.console.w + x];
                            if y < h && x < w {
                                grid[y * w + x] = block;
                            }

                            window.rect(x as i32 * 8, y as i32 * 16, w as u32 * 8, h as u32 * 16, Color { data: block.bg });
                            if block.c != '\0' {
                                if block.bold {
                                    font_bold.render(&block.c.encode_utf8(&mut str_buf), 16.0).draw(window, x as i32 * 8, y as i32 * 16, Color { data: block.fg });
                                } else {
                                    font.render(&block.c.encode_utf8(&mut str_buf), 16.0).draw(window, x as i32 * 8, y as i32 * 16, Color { data: block.fg });
                                }
                            }
                        }
                    }
                }

                self.console.w = w;
                self.console.h = h;
                self.grid = grid;

                if self.console.cursor && self.console.x < self.console.w && self.console.y < self.console.h {
                    let x = self.console.x;
                    let y = self.console.y;
                    self.invert(x * 8, y * 16, 8, 16);
                }

                self.redraw();
            },
            _ => ()
        }
    }

    pub fn invert(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let width = self.window.width() as usize;
        let height = self.window.height() as usize;

        let start_y = cmp::min(height - 1, y);
        let end_y = cmp::min(height, y + h);

        let start_x = cmp::min(width - 1, x);
        let len = cmp::min(width, x + w) - start_x;

        let mut offscreen_ptr = self.window.data_mut().as_mut_ptr() as usize;

        let stride = width * 4;

        let offset = y * stride + start_x * 4;
        offscreen_ptr += offset;

        let mut rows = end_y - start_y;
        while rows > 0 {
            let mut row_ptr = offscreen_ptr;
            let mut cols = len;
            while cols > 0 {
                unsafe {
                    let color = *(row_ptr as *mut u32);
                    *(row_ptr as *mut u32) = color ^ 0x00FFFFFF;
                }
                row_ptr += 4;
                cols -= 1;
            }
            offscreen_ptr += stride;
            rows -= 1;
        }
    }

    pub fn write(&mut self, buf: &[u8], sync: bool) -> Result<usize> {
        if self.console.cursor && self.console.x < self.console.w && self.console.y < self.console.h {
            let x = self.console.x;
            let y = self.console.y;
            self.invert(x * 8, y * 16, 8, 16);
            self.changed.insert(y);
        }

        {
            let font = &self.font;
            let font_bold = &self.font_bold;
            let console_w = self.console.w;
            let grid = &mut self.grid;
            let window = &mut self.window;
            let changed = &mut self.changed;
            let mut str_buf = [0; 4];
            self.console.write(buf, |event| {
                match event {
                    ransid::Event::Char { x, y, c, color, bold, .. } => {
                        {
                            let block = &mut grid[y * console_w + x];
                            block.c = c;
                            block.fg = color.data;
                            block.bold = bold;
                        }
                        if bold {
                            font_bold.render(&c.encode_utf8(&mut str_buf), 16.0).draw(window, x as i32 * 8, y as i32 * 16, Color { data: color.data });
                        } else {
                            font.render(&c.encode_utf8(&mut str_buf), 16.0).draw(window, x as i32 * 8, y as i32 * 16, Color { data: color.data });
                        }
                        changed.insert(y);
                    },
                    ransid::Event::Rect { x, y, w, h, color } => {
                        {
                            let block = &mut grid[y * console_w + x];
                            block.c = '\0';
                            block.bg = color.data;
                            block.bold = false;
                        }
                        window.rect(x as i32 * 8, y as i32 * 16, w as u32 * 8, h as u32 * 16, Color { data: color.data });
                        for y2 in y..y + h {
                            changed.insert(y2);
                        }
                    },
                    ransid::Event::Scroll { rows, color } => {
                        let rows = rows as u32 * 16;

                        let width = window.width();
                        let height = window.height();
                        if rows > 0 && rows < height {
                            let off1 = rows * width;
                            let off2 = height * width - off1;
                            unsafe {
                                let data_ptr = window.data_mut().as_mut_ptr() as *mut u32;
                                fast_copy(data_ptr as *mut u8, data_ptr.offset(off1 as isize) as *const u8, off2 as usize * 4);
                                fast_set32(data_ptr.offset(off2 as isize), color.data, off1 as usize);
                            }
                        }

                        for y in 0..window.height()/16 {
                            changed.insert(y as usize);
                        }
                    },
                    ransid::Event::Title { title } => {
                        window.set_title(&title);
                    }
                }
            });
        }

        if self.console.cursor && self.console.x < self.console.w && self.console.y < self.console.h {
            let x = self.console.x;
            let y = self.console.y;
            self.invert(x * 8, y * 16, 8, 16);
            self.changed.insert(y as usize);
        }

        if ! self.console.raw_mode && sync {
            self.sync();
        }

        Ok(buf.len())
    }

    fn sync(&mut self) {
        /*
        let width = self.window.width;
        for change in self.changed.iter() {
            self.display.sync(0, change * 16, width, 16);
        }
        */
        if ! self.changed.is_empty() {
            self.window.sync();
        }
        self.changed.clear();
    }

    pub fn redraw(&mut self) {
        /*
        let width = self.window.width;
        let height = self.window.height;
        */
        self.window.sync();
        self.changed.clear();
    }
}
