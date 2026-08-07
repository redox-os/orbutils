#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

use log::{error, info};
use orbclient::rect::Rect;
use std::cell::{Cell, OnceCell};
use std::error::Error;
use std::num::NonZero;
use std::ops::ControlFlow;
use std::process::Command;
use std::{env, io, str};
use winit::application::ApplicationHandler;
use winit::event::{ButtonSource, MouseButton, WindowEvent};
use winit::event_loop::run_on_demand::EventLoopExtRunOnDemand;
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::keyboard::{Key, KeyCode, PhysicalKey};
use winit::monitor::Fullscreen;
use winit::window::{Window, WindowAttributes, WindowId};

use orbclient::image::Image;
use orbclient::{Color, Mode, Renderer};
use orbfont::Font;
use redox_log::{OutputBuilder, RedoxLogger};
use redox_users::{All, AllUsers, Config};

mod keymap;

#[derive(Clone, Copy)]
enum BackgroundMode {
    /// Do not resize the image, just center it
    Center,
    /// Resize the image to the display size
    Fill,
    /// Resize the image - keeping its aspect ratio, and fit it to the display with blank space
    Scale,
    /// Resize the image - keeping its aspect ratio, and crop to remove all blank space
    Zoom,
}

impl BackgroundMode {
    fn from_str(string: &str) -> BackgroundMode {
        match string {
            "fill" => BackgroundMode::Fill,
            "scale" => BackgroundMode::Scale,
            "zoom" => BackgroundMode::Zoom,
            _ => BackgroundMode::Center,
        }
    }
}

fn find_scale(
    image: &Image,
    mode: BackgroundMode,
    display_width: u32,
    display_height: u32,
) -> (u32, u32) {
    match mode {
        BackgroundMode::Center => (image.width(), image.height()),
        BackgroundMode::Fill => (display_width, display_height),
        BackgroundMode::Scale => {
            let d_w = display_width as f64;
            let d_h = display_height as f64;
            let i_w = image.width() as f64;
            let i_h = image.height() as f64;

            let scale = if d_w / d_h > i_w / i_h {
                d_h / i_h
            } else {
                d_w / i_w
            };

            ((i_w * scale) as u32, (i_h * scale) as u32)
        }
        BackgroundMode::Zoom => {
            let d_w = display_width as f64;
            let d_h = display_height as f64;
            let i_w = image.width() as f64;
            let i_h = image.height() as f64;

            let scale = if d_w / d_h < i_w / i_h {
                d_h / i_h
            } else {
                d_w / i_w
            };

            ((i_w * scale) as u32, (i_h * scale) as u32)
        }
    }
}

fn normal_usernames() -> Vec<String> {
    let users = match AllUsers::authenticator(Config::default()) {
        Ok(ok) => ok,
        Err(_) => return Vec::new(),
    };

    let mut usernames = Vec::new();
    for user in users.iter() {
        if user.uid >= 1000 {
            usernames.push(user.user.clone());
        }
    }
    usernames.sort();
    usernames
}

fn login_command(
    username: &str,
    pass: &str,
    launcher_cmd: &str,
    launcher_args: &[String],
) -> Option<(Command, String)> {
    let sys_users = match AllUsers::authenticator(Config::default()) {
        Ok(users) => users,
        // Not maybe the best thing to do...
        Err(_) => return None,
    };

    match sys_users.get_by_name(&username) {
        Some(user) => {
            if user.verify_passwd(&pass) {
                let mut command = user.login_cmd(&launcher_cmd);
                for arg in launcher_args.iter() {
                    command.arg(&arg);
                }

                Some((command, user.user.clone()))
            } else {
                None
            }
        }
        None => None,
    }
}

struct UiConfig {
    font: Font,

    image_mode: BackgroundMode,
    image: Image,

    btn_color_inactive: Color,
    btn_color_active: Color,
    btn_border_color: Color,
}

fn build_ui_config() -> Result<UiConfig, Box<dyn Error + 'static>> {
    // FIXME replace
    const IMAGE_PATH: &str = if cfg!(target_os = "redox") {
        "/usr/share/ui/login.png"
    } else {
        "/usr/share/icons/desktop-base/128x128/emblems/emblem-debian.png"
    };

    Ok(UiConfig {
        font: Font::find(Some("Sans"), None, None)?,

        image_mode: BackgroundMode::from_str("zoom"),
        image: Image::from_path(IMAGE_PATH).map_err(|_| format!("Cannot load {}", IMAGE_PATH))?,

        btn_color_inactive: Color::rgb(39, 72, 105),
        btn_color_active: Color::rgb(59, 102, 135),
        btn_border_color: Color::rgb(29, 29, 29),
    })
}

struct UiLayout {
    s_u: u32,
    s_i: i32,
    s_f: f32,

    btn_size_u: u32,
    btn_size_i: i32,
    padding: i32,
    btn_inner_x: i32,
    btn_inner_y: i32,
    btn_inner_w: u32,
    btn_inner_h: u32,
    btn_text_offset_y: i32,
    item_height_u: u32,
    item_height_i: i32,
    menu_width_u: u32,
    menu_width_i: i32,
}

fn build_ui_layout() -> Result<UiLayout, Box<dyn Error + 'static>> {
    let (_display_width, display_height) = orbclient::get_display_size()?;
    let s_u = (display_height / 1600) + 1;
    let s_i = s_u as i32;
    let s_f = s_i as f32;

    let btn_size_u = 28 * s_u;
    let btn_size_i = 28 * s_i;
    Ok(UiLayout {
        s_u,
        s_i,
        s_f,

        btn_size_u,
        btn_size_i,
        padding: 10 * s_i,
        btn_inner_x: 2 * s_i,
        btn_inner_y: 2 * s_i,
        btn_inner_w: (btn_size_i - 4 * s_i) as u32,
        btn_inner_h: (btn_size_i - 4 * s_i) as u32,
        btn_text_offset_y: 6 * s_i,
        item_height_u: 28 * s_u,
        item_height_i: 28 * s_i,
        menu_width_u: 150 * s_u,
        menu_width_i: 150 * s_i,
    })
}

struct UiState {
    item: i32,
    username: String,
    password: String,
    failure: bool,
    keymap_dropdown_open: bool,
    power_dropdown_open: bool,
    keymap_state: keymap::KeymapState,
    keymap_options: Vec<String>,
    power_options: Vec<&'static str>,
    mouse_x: i32,
    mouse_y: i32,
    mouse_left: bool,
}

type Surface = softbuffer::Surface<OwnedDisplayHandle, Box<dyn Window + 'static>>;

// General surface
pub struct OrbBuffer<'a> {
    w: u32,
    h: u32,
    mode: Cell<Mode>,
    data: &'a mut [Color],
}

impl<'a> OrbBuffer<'a> {
    fn from_buffer(buffer: &mut softbuffer::Buffer<'a>) -> Self {
        let w = buffer.width().get();
        let h = buffer.height().get();
        let raw = buffer.data_u8();
        let data = unsafe {
            let raw_pixels = core::ptr::from_mut(raw) as *mut Color;
            std::slice::from_raw_parts_mut(raw_pixels, raw.len() / 4)
        };
        OrbBuffer {
            w,
            h,
            mode: Cell::new(Mode::Blend),
            data,
        }
    }
}

impl Renderer for OrbBuffer<'_> {
    fn width(&self) -> u32 {
        self.w
    }

    fn height(&self) -> u32 {
        self.h
    }

    fn data(&self) -> &[Color] {
        self.data
    }

    fn data_mut(&mut self) -> &mut [Color] {
        self.data
    }

    fn sync(&mut self) -> bool {
        true
    }

    fn update(&mut self) -> bool {
        true
    }

    fn update_rects(&mut self, _rects: &[(i32, i32, u32, u32)]) -> bool {
        true
    }

    fn mode(&self) -> &Cell<Mode> {
        &self.mode
    }
}

struct App<'a> {
    launcher_cmd: &'a str,
    launcher_args: &'a [String],
    context: &'a softbuffer::Context<OwnedDisplayHandle>,
    ui_config: UiConfig,
    ui_layout: UiLayout,
    ui_state: UiState,
    scaled_image: Image,
    window: OnceCell<Surface>,
    output: &'a mut Option<(Command, String)>,
}

impl ApplicationHandler for App<'_> {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let window = event_loop
            .create_window(
                WindowAttributes::default().with_fullscreen(Some(Fullscreen::Borderless(None))),
            )
            .unwrap();
        let mut surface = softbuffer::Surface::new(&self.context, window).unwrap();
        surface
            .configure(
                NonZero::new(surface.window().outer_size().width).unwrap(),
                NonZero::new(surface.window().outer_size().height).unwrap(),
                softbuffer::AlphaMode::Ignored,
            )
            .unwrap();
        self.window.set(surface).unwrap();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::SurfaceResized(size) => {
                self.window
                    .get_mut()
                    .unwrap()
                    .resize(
                        NonZero::new(size.width).unwrap(),
                        NonZero::new(size.height).unwrap(),
                    )
                    .unwrap();
                self.window.get().unwrap().window().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.get_mut().unwrap();
                let mut buffer = window.next_buffer().unwrap();

                handle_redraw(
                    &self.ui_config,
                    &self.ui_layout,
                    &mut OrbBuffer::from_buffer(&mut buffer),
                    &mut self.scaled_image,
                    &self.ui_state,
                );

                buffer.present().unwrap();
            }
            event => {
                match handle_event(
                    self.launcher_cmd,
                    self.launcher_args,
                    &self.ui_layout,
                    &**self.window.get().unwrap().window(),
                    &mut self.ui_state,
                    event,
                ) {
                    ControlFlow::Break(res) => {
                        *self.output = res;
                        event_loop.exit();
                    }
                    ControlFlow::Continue(()) => {}
                }
            }
        }
    }
}

fn login_window(
    launcher_cmd: &str,
    launcher_args: &[String],
    event_loop: &mut EventLoop,
    context: &softbuffer::Context<OwnedDisplayHandle>,
) -> Result<Option<(Command, String)>, Box<dyn Error>> {
    let ui_config = build_ui_config()?;
    let ui_layout = build_ui_layout()?;

    let (item, username) = match *normal_usernames() {
        [ref username] => (1, username.clone()),
        _ => (0, String::new()),
    };
    let keymap_state = crate::keymap::KeymapState::new();
    let keymap_options = keymap_state.list.clone();
    let ui_state = UiState {
        item,
        username,
        password: String::new(),
        failure: false,
        keymap_dropdown_open: false,
        power_dropdown_open: false,
        keymap_state,
        keymap_options,
        power_options: vec!["Restart", "Shutdown"],
        mouse_x: 0,
        mouse_y: 0,
        mouse_left: false,
    };
    let scaled_image = ui_config.image.clone();

    let mut output = None;

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app_on_demand(App {
        launcher_cmd,
        launcher_args,
        context,
        ui_config,
        ui_layout,
        ui_state,
        scaled_image,
        window: OnceCell::new(),
        output: &mut output,
    })?;

    Ok(output)
}

fn handle_redraw(
    ui_config: &UiConfig,
    ui_layout: &UiLayout,
    window: &mut OrbBuffer<'_>,
    scaled_image: &mut Image,
    ui_state: &UiState,
) {
    let UiConfig {
        ref font,
        image_mode,
        ref image,
        btn_color_inactive,
        btn_color_active,
        btn_border_color,
        ..
    } = *ui_config;

    let UiLayout {
        s_u,
        s_i,
        s_f,
        btn_size_u,
        btn_size_i,
        padding,
        btn_inner_x,
        btn_inner_y,
        btn_inner_w,
        btn_inner_h,
        btn_text_offset_y,
        item_height_u,
        item_height_i,
        menu_width_u,
        menu_width_i,
        ..
    } = *ui_layout;

    let UiState {
        item,
        ref username,
        ref password,
        failure,
        keymap_dropdown_open,
        power_dropdown_open,
        ref keymap_state,
        ref keymap_options,
        ref power_options,
        mouse_x,
        mouse_y,
        ..
    } = *ui_state;

    let (w, h) = (window.width(), window.height());

    let (width, height) = find_scale(&image, image_mode, w, h);

    if width == scaled_image.width() && height == scaled_image.height() {
        // Do not resize scaled image
    } else if width == image.width() && height == image.height() {
        *scaled_image = image.clone();
    } else {
        *scaled_image = image.resize(width, height, orbclient::image::ResizeType::Lanczos3);
    }

    let (crop_x, crop_w) = if width > w {
        ((width - w) / 2, w)
    } else {
        (0, width)
    };

    let (crop_y, crop_h) = if height > h {
        ((height - h) / 2, h)
    } else {
        (0, height)
    };

    window.set(Color::rgb(0, 0, 0));

    let x = (w as i32 - crop_w as i32) / 2;
    let y = (h as i32 - crop_h as i32) / 2;
    scaled_image
        .roi(&Rect::new(crop_x as i32, crop_y as i32, crop_w, crop_h))
        .draw(window, x, y);

    let x = (window.width() as i32 - 216 * s_i) / 2;
    let y = (window.height() as i32 - 164 * s_i) / 2;
    window.rect(x, y, 216 * s_u, 164 * s_u, Color::rgba(0, 0, 0, 128));

    font.render("Username:", 16.0 * s_f).draw(
        window,
        x + 8 * s_i,
        y + 8 * s_i,
        Color::rgb(255, 255, 255),
    );
    font.render("Password:", 16.0 * s_f).draw(
        window,
        x + 8 * s_i,
        y + 68 * s_i,
        Color::rgb(255, 255, 255),
    );

    let active = if failure {
        Color::rgb(255, 0, 0)
    } else {
        Color::rgb(255, 255, 255)
    };
    let inactive = if failure {
        Color::rgb(128, 0, 0)
    } else {
        Color::rgb(29, 29, 29)
    };

    let x = (window.width() as i32 - 200 * s_i) / 2;
    let mut y = (window.height() as i32 - 148 * s_i) / 2;

    y += 24 * s_i;

    // --- Username Box ---
    {
        window.rect(
            x,
            y,
            200 * s_u,
            28 * s_u,
            if item == 0 { active } else { inactive },
        );
        window.rect(
            x + 2 * s_i,
            y + 2 * s_i,
            196 * s_u,
            24 * s_u,
            Color::rgb(40, 40, 40),
        );
        let mut string = username.to_string();
        if item == 0 {
            string.push('|');
        }
        font.render(&string, 16.0 * s_f).draw(
            window,
            x + 6 * s_i,
            y + 6 * s_i,
            Color::rgb(255, 255, 255),
        );
    }

    y += 60 * s_i;

    // --- Password Box ---
    {
        window.rect(
            x,
            y,
            200 * s_u,
            28 * s_u,
            if item == 1 { active } else { inactive },
        );
        window.rect(
            x + 2 * s_i,
            y + 2 * s_i,
            196 * s_u,
            24 * s_u,
            Color::rgb(40, 40, 40),
        );
        let mut string = String::new();
        for _c in password.chars() {
            string.push('•');
        }
        if item == 1 {
            string.push('|');
        }
        font.render(&string, 16.0 * s_f).draw(
            window,
            x + 6 * s_i,
            y + 6 * s_i,
            Color::rgb(255, 255, 255),
        );
    }

    y += 36 * s_i;

    // --- Login Button ---
    {
        window.rect(x, y, 200 * s_u, 28 * s_u, Color::rgb(29, 29, 29));
        window.rect(
            x + 2 * s_i,
            y + 2 * s_i,
            196 * s_u,
            24 * s_u,
            Color::rgb(39, 72, 105),
        );
        let text = font.render(&"Login", 16.0 * s_f);
        text.draw(
            window,
            x + (200 * s_i - text.width() as i32) / 2,
            y + 6 * s_i,
            Color::rgb(255, 255, 255),
        );
    }

    // --- Buttons and Menus ---
    {
        let power_btn_y = window.height() as i32 - btn_size_i - padding;
        let power_btn_x = window.width() as i32 - btn_size_i - padding;
        let keymap_btn_y = power_btn_y;
        let keymap_btn_x = power_btn_x - btn_size_i - padding;

        let keymap_hover = mouse_x >= keymap_btn_x
            && mouse_x < keymap_btn_x + btn_size_i
            && mouse_y >= keymap_btn_y
            && mouse_y < keymap_btn_y + btn_size_i;
        let power_hover = mouse_x >= power_btn_x
            && mouse_x < power_btn_x + btn_size_i
            && mouse_y >= power_btn_y
            && mouse_y < power_btn_y + btn_size_i;

        let keymap_color = if keymap_hover || keymap_dropdown_open {
            btn_color_active
        } else {
            btn_color_inactive
        };
        let power_color = if power_hover || power_dropdown_open {
            btn_color_active
        } else {
            btn_color_inactive
        };

        {
            window.rect(
                keymap_btn_x,
                keymap_btn_y,
                btn_size_u,
                btn_size_u,
                btn_border_color,
            );
            window.rect(
                keymap_btn_x + btn_inner_x,
                keymap_btn_y + btn_inner_y,
                btn_inner_w,
                btn_inner_h,
                keymap_color,
            );
            let text_k = font.render("K", 16.0 * s_f);
            text_k.draw(
                window,
                keymap_btn_x + (btn_size_i - text_k.width() as i32) / 2,
                keymap_btn_y + btn_text_offset_y,
                Color::rgb(255, 255, 255),
            );
        }
        {
            window.rect(
                power_btn_x,
                power_btn_y,
                btn_size_u,
                btn_size_u,
                btn_border_color,
            );
            window.rect(
                power_btn_x + btn_inner_x,
                power_btn_y + btn_inner_y,
                btn_inner_w,
                btn_inner_h,
                power_color,
            );
            let text_p = font.render("P", 16.0 * s_f);
            text_p.draw(
                window,
                power_btn_x + (btn_size_i - text_p.width() as i32) / 2,
                power_btn_y + btn_text_offset_y,
                Color::rgb(255, 255, 255),
            );
        }
        if keymap_dropdown_open {
            let menu_height_u = item_height_u * keymap_options.len() as u32;
            let menu_x = keymap_btn_x - (menu_width_i - btn_size_i); // Align right
            let menu_y = keymap_btn_y - menu_height_u as i32; // Opens upwards

            window.rect(
                menu_x,
                menu_y,
                menu_width_u,
                menu_height_u,
                btn_border_color,
            );

            for (i, option) in keymap_options.iter().enumerate() {
                let item_y = menu_y + (i as i32 * item_height_i);
                let item_hover = mouse_x >= menu_x
                    && mouse_x < menu_x + menu_width_i
                    && mouse_y >= item_y
                    && mouse_y < item_y + item_height_i;
                let item_color = if item_hover || option == &keymap_state.active {
                    btn_color_active
                } else {
                    btn_color_inactive
                };
                window.rect(
                    menu_x + btn_inner_x,
                    item_y + btn_inner_y,
                    (menu_width_i - 4 * s_i) as u32,
                    (item_height_i - 4 * s_i) as u32,
                    item_color,
                );
                font.render(option, 16.0 * s_f).draw(
                    window,
                    menu_x + 6 * s_i,
                    item_y + 6 * s_i,
                    Color::rgb(255, 255, 255),
                );
            }
        }

        if power_dropdown_open {
            let menu_height_u = item_height_u * power_options.len() as u32;
            let menu_x = power_btn_x - (menu_width_i - btn_size_i); // Align right
            let menu_y = power_btn_y - menu_height_u as i32; // Opens upwards

            window.rect(
                menu_x,
                menu_y,
                menu_width_u,
                menu_height_u,
                btn_border_color,
            );

            for (i, option) in power_options.iter().enumerate() {
                let item_y = menu_y + (i as i32 * item_height_i);
                let item_hover = mouse_x >= menu_x
                    && mouse_x < menu_x + menu_width_i
                    && mouse_y >= item_y
                    && mouse_y < item_y + item_height_i;
                let item_color = if item_hover {
                    btn_color_active
                } else {
                    btn_color_inactive
                };
                window.rect(
                    menu_x + btn_inner_x,
                    item_y + btn_inner_y,
                    (menu_width_i - 4 * s_i) as u32,
                    (item_height_i - 4 * s_i) as u32,
                    item_color,
                );
                font.render(option, 16.0 * s_f).draw(
                    window,
                    menu_x + 6 * s_i,
                    item_y + 6 * s_i,
                    Color::rgb(255, 255, 255),
                );
            }
        }
    }

    window.sync();
}

fn handle_event(
    launcher_cmd: &str,
    launcher_args: &[String],
    ui_layout: &UiLayout,
    window: &dyn Window,
    ui_state: &mut UiState,
    event: WindowEvent,
) -> ControlFlow<Option<(Command, String)>> {
    let UiLayout {
        s_i,
        btn_size_i,
        padding,
        item_height_i,
        menu_width_i,
        ..
    } = *ui_layout;

    let UiState {
        item,
        username,
        password,
        failure,
        keymap_dropdown_open,
        power_dropdown_open,
        keymap_state,
        keymap_options,
        power_options,
        mouse_x,
        mouse_y,
        mouse_left,
    } = ui_state;

    match event {
        WindowEvent::KeyboardInput {
            device_id: _,
            event,
            is_synthetic: _,
        } => {
            if !event.state.is_pressed() {
                return ControlFlow::Continue(());
            }
            if let Key::Character(text) = event.logical_key {
                if *item == 0 {
                    username.push_str(&*text);
                } else if *item == 1 {
                    password.push_str(&*text);
                }

                window.request_redraw();
            } else {
                if *keymap_dropdown_open || *power_dropdown_open {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Enter)
                        | PhysicalKey::Code(KeyCode::Escape)
                        | PhysicalKey::Code(KeyCode::Tab) => {
                            *keymap_dropdown_open = false;
                            *power_dropdown_open = false;
                            window.request_redraw();
                        }
                        _ => (),
                    }
                }

                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Backspace) => {
                        if *item == 0 {
                            username.pop();
                        } else if *item == 1 {
                            password.pop();
                        }

                        window.request_redraw();
                    }
                    PhysicalKey::Code(KeyCode::Enter) => {
                        if *item == 0 {
                            *item = 1;
                        } else if *item == 1 {
                            if let Some(command) =
                                login_command(&*username, &*password, launcher_cmd, launcher_args)
                            {
                                return ControlFlow::Break(Some(command));
                            } else {
                                *item = 0;
                                password.clear();
                                *failure = true
                            }
                        }

                        window.request_redraw();
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        *item = 0;
                        username.clear();
                        password.clear();
                        *failure = false;

                        window.request_redraw();
                    }
                    PhysicalKey::Code(KeyCode::Tab) => {
                        if *item == 0 {
                            *item = 1;
                        } else if *item == 1 {
                            *item = 0;
                        }

                        window.request_redraw();
                    }
                    _ => {}
                }
            }
        }
        WindowEvent::PointerMoved {
            device_id: _,
            position,
            primary: true,
            source: _,
        } => {
            *mouse_x = position.x as i32;
            *mouse_y = position.y as i32;

            window.request_redraw();
        }
        WindowEvent::PointerButton {
            device_id: _,
            state,
            position: _, // FIXME (0,0) on orbital
            primary: true,
            button: ButtonSource::Mouse(MouseButton::Left),
            is_macos_activation_click: _,
        } => {
            if !state.is_pressed() && *mouse_left {
                let power_btn_y = window.outer_size().height as i32 - btn_size_i - padding;
                let power_btn_x = window.outer_size().width as i32 - btn_size_i - padding;
                let keymap_btn_y = power_btn_y;
                let keymap_btn_x = power_btn_x - btn_size_i - padding;

                let trigger_redraw = || {
                    window.request_redraw();
                };

                if *keymap_dropdown_open {
                    let menu_height_i = item_height_i * keymap_options.len() as i32;
                    let menu_x = keymap_btn_x - (menu_width_i - btn_size_i); // Align right
                    let menu_y = keymap_btn_y - menu_height_i;

                    if *mouse_x >= menu_x
                        && *mouse_x < menu_x + menu_width_i
                        && *mouse_y >= menu_y
                        && *mouse_y < menu_y + menu_height_i
                    {
                        let item_index = (*mouse_y - menu_y) / item_height_i;
                        if let Some(option) = keymap_options.get(item_index as usize) {
                            keymap_state.set_active(option);
                        }
                        *keymap_dropdown_open = false;
                        trigger_redraw();
                        return ControlFlow::Continue(());
                    }
                }

                if *power_dropdown_open {
                    let menu_height_i = item_height_i * power_options.len() as i32;
                    let menu_x = power_btn_x - (menu_width_i - btn_size_i); // Align right
                    let menu_y = power_btn_y - menu_height_i;

                    if *mouse_x >= menu_x
                        && *mouse_x < menu_x + menu_width_i
                        && *mouse_y >= menu_y
                        && *mouse_y < menu_y + menu_height_i
                    {
                        let item_index = (*mouse_y - menu_y) / item_height_i;
                        if let Some(option) = power_options.get(item_index as usize) {
                            if *option == "Shutdown" {
                                Command::new("shutdown").spawn().ok();
                            } else if *option == "Restart" {
                                Command::new("shutdown").arg("-r").spawn().ok();
                            }
                        }
                        *power_dropdown_open = false;
                        trigger_redraw();
                        return ControlFlow::Continue(());
                    }
                }

                if *mouse_x >= keymap_btn_x
                    && *mouse_x < keymap_btn_x + btn_size_i
                    && *mouse_y >= keymap_btn_y
                    && *mouse_y < keymap_btn_y + btn_size_i
                {
                    *keymap_dropdown_open = !*keymap_dropdown_open;
                    *power_dropdown_open = false;
                    trigger_redraw();
                    return ControlFlow::Continue(());
                } else if *mouse_x >= power_btn_x
                    && *mouse_x < power_btn_x + btn_size_i
                    && *mouse_y >= power_btn_y
                    && *mouse_y < power_btn_y + btn_size_i
                {
                    *power_dropdown_open = !*power_dropdown_open;
                    *keymap_dropdown_open = false;
                    trigger_redraw();
                    return ControlFlow::Continue(());
                } else {
                    let x = (window.outer_size().width as i32 - 216 * s_i) / 2;
                    let y = (window.outer_size().height as i32 - 164 * s_i) / 2;

                    if *mouse_x >= x
                        && *mouse_x < x + 216 * s_i
                        && *mouse_y >= y
                        && *mouse_y < y + 164 * s_i
                    {
                        if *mouse_y < y + 64 * s_i {
                            *item = 0;
                        } else if *mouse_y < y + 128 * s_i {
                            *item = 1;
                        } else {
                            if let Some(command) =
                                login_command(&*username, &*password, launcher_cmd, launcher_args)
                            {
                                return ControlFlow::Break(Some(command));
                            } else {
                                *item = 0;
                                password.clear();
                                *failure = true
                            }
                        }
                        trigger_redraw();
                        return ControlFlow::Continue(());
                    }
                }

                if *keymap_dropdown_open || *power_dropdown_open {
                    *keymap_dropdown_open = false;
                    *power_dropdown_open = false;
                    trigger_redraw();
                    return ControlFlow::Continue(());
                }
            }
            *mouse_left = state.is_pressed();
        }
        _ => (),
    }

    ControlFlow::Continue(())
}

fn main() -> io::Result<()> {
    // Ignore possible errors while enabling logging
    let _ = RedoxLogger::new()
        .with_output(
            OutputBuilder::stdout()
                .with_filter(log::LevelFilter::Warn)
                .with_ansi_escape_codes()
                .build(),
        )
        .with_process_name("orblogin".into())
        .enable();

    let mut args = env::args().skip(1);

    let launcher_cmd = args.next().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Could not get 'launcher_cmd'",
    ))?;
    let launcher_args: Vec<String> = args.collect();

    let mut event_loop = EventLoop::new().unwrap();
    let context = softbuffer::Context::new(event_loop.owned_display_handle()).unwrap();

    loop {
        match login_window(&launcher_cmd, &launcher_args, &mut event_loop, &context) {
            Ok(Some((mut command, username))) => {
                #[cfg(target_os = "redox")]
                let before_ns_fd = {
                    let Ok(before_ns_fd) =
                        redox::apply_login_schemes(username, &redox::DEFAULT_SCHEMES)
                    else {
                        continue;
                    };
                    let _ = libredox::call::fcntl(
                        before_ns_fd.raw(),
                        syscall::F_SETFD,
                        libredox::flag::O_CLOEXEC as usize,
                    );
                    before_ns_fd
                };
                match command.spawn() {
                    Ok(mut child) => match child.wait() {
                        Ok(_) => (),
                        Err(err) => error!("failed to wait for '{}': {}", launcher_cmd, err),
                    },
                    Err(err) => error!("failed to execute '{}': {}", launcher_cmd, err),
                }
                #[cfg(target_os = "redox")]
                {
                    let _ = libredox::call::fcntl(before_ns_fd.raw(), syscall::F_SETFD, 0);
                    let _ = libredox::call::close(
                        libredox::call::setns(before_ns_fd.into_raw())
                            .expect("failed to restore namespace"),
                    );
                }
            }
            Ok(None) => info!("login completed without a command"),
            Err(e) => error!("{}", e),
        }
    }
}

// TODO: Move to redox_users once the definition solidifies.
#[cfg(target_os = "redox")]
mod redox {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use std::fs;

    pub const DEFAULT_SCHEMES: [&'static str; 27] = [
        // Kernel schemes
        "debug",
        "event",
        "memory",
        "pipe",
        "serio",
        "irq",
        "time",
        "sys",
        // Base schemes
        "rand",
        "null",
        "zero",
        "log",
        // Network schemes
        "ip",
        "icmp",
        "tcp",
        "udp",
        // IPC schemes
        "shm",
        "chan",
        "uds_stream",
        "uds_dgram",
        // File schemes
        "file",
        // Display schemes
        "display.vesa",
        "display*",
        // Other schemes
        "pty",
        "sudo",
        "audio",
        "orbital",
    ];

    const LOGIN_SCHEMES_FILE: &'static str = "/etc/login_schemes.toml";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UserSchemeConfig {
        pub schemes: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct LoginConfig {
        #[serde(rename = "user_schemes")]
        pub user_schemes: BTreeMap<String, UserSchemeConfig>,
    }

    pub fn apply_login_schemes(
        user: String,
        default_schemes: &[&str],
    ) -> libredox::error::Result<libredox::Fd> {
        let schemes = match load_config_schemes(&user) {
            Some(s) => s,
            _ => default_schemes.iter().map(|s| s.to_string()).collect(),
        };

        let mut names: Vec<ioslice::IoSlice> = Vec::with_capacity(schemes.len());
        for scheme in schemes.iter() {
            names.push(ioslice::IoSlice::new(scheme.as_bytes()));
        }

        let ns_fd = libredox::call::mkns(&names)?;
        let before_ns_fd = libredox::Fd::new(libredox::call::setns(ns_fd)?);

        Ok(before_ns_fd)
    }

    fn load_config_schemes(user: &str) -> Option<Vec<String>> {
        let config_str = fs::read_to_string(LOGIN_SCHEMES_FILE).ok()?;
        let config: LoginConfig = toml::from_str(&config_str).ok()?;

        config.user_schemes.get(user).map(|cfg| cfg.schemes.clone())
    }
}
