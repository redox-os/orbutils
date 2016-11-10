extern crate ransid;

use std::cmp;
use std::collections::{BTreeSet, VecDeque};

use orbclient::{Color, Event, EventOption, Window};
use syscall::Result;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_copy64(dst: *mut u64, src: *const u64, len: usize) {
    asm!("cld
        rep movsq"
        :
        : "{rdi}"(dst as usize), "{rsi}"(src as usize), "{rcx}"(len)
        : "cc", "memory", "rdi", "rsi", "rcx"
        : "intel", "volatile");
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_set64(dst: *mut u64, src: u64, len: usize) {
    asm!("cld
        rep stosq"
        :
        : "{rdi}"(dst as usize), "{rax}"(src), "{rcx}"(len)
        : "cc", "memory", "rdi", "rcx"
        : "intel", "volatile");
}

pub struct Console {
    pub console: ransid::Console,
    pub window: Window,
    pub changed: BTreeSet<usize>,
    pub ctrl: bool,
    pub input: Vec<u8>,
    pub end_of_input: bool,
    pub cooked: VecDeque<u8>,
    pub requested: usize
}

impl Console {
    pub fn new(width: u32, height: u32) -> Console {
        let mut window = Window::new_flags(-1, -1, width, height, "Terminal", true).unwrap();
        window.sync();
        Console {
            console: ransid::Console::new(width as usize / 8, height as usize / 16),
            window: window,
            changed: BTreeSet::new(),
            ctrl: false,
            input: Vec::new(),
            end_of_input: false,
            cooked: VecDeque::new(),
            requested: 0
        }
    }

    pub fn input(&mut self, event: &Event) {
        let mut buf = vec![];

        match event.to_option() {
            EventOption::Key(key_event) => {
                if key_event.scancode == 0x1D {
                    self.ctrl = key_event.pressed;
                } else if key_event.pressed {
                    match key_event.scancode {
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
            },
            _ => () //TODO: Mouse in terminal
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
                    *(row_ptr as *mut u32) = !color;
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
            let window = &mut self.window;
            let changed = &mut self.changed;
            self.console.write(buf, |event| {
                match event {
                    ransid::Event::Char { x, y, c, color, .. } => {
                        window.char(x as i32 * 8, y as i32 * 16, c, Color { data: color.data });/*, bold, false);*/
                        changed.insert(y);
                    },
                    ransid::Event::Rect { x, y, w, h, color } => {
                        window.rect(x as i32 * 8, y as i32 * 16, w as u32 * 8, h as u32 * 16, Color { data: color.data });
                        for y2 in y..y + h {
                            changed.insert(y2);
                        }
                    },
                    ransid::Event::Scroll { rows, color } => {
                        let rows = rows as u32 * 16;
                        let data = (color.data as u64) << 32 | color.data as u64;

                        let width = window.width()/2;
                        let height = window.height();
                        if rows > 0 && rows < height {
                            let off1 = rows * width;
                            let off2 = height * width - off1;
                            unsafe {
                                let data_ptr = window.data_mut().as_mut_ptr() as *mut u64;
                                fast_copy64(data_ptr, data_ptr.offset(off1 as isize), off2 as usize);
                                fast_set64(data_ptr.offset(off2 as isize), data, off1 as usize);
                            }
                        }

                        for y in 0..window.height()/16 {
                            changed.insert(y as usize);
                        }
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
