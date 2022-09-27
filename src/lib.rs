use std::{
    env,
    fs::File,
    os::unix::io::{AsRawFd, FromRawFd, RawFd},
};

pub struct DisplayRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

fn get_full_url(path: &str) -> Result<String, String> {
    let file = match syscall::open(path, syscall::O_CLOEXEC | syscall::O_STAT) {
        Ok(ok) => unsafe { File::from_raw_fd(ok as RawFd) },
        Err(err) => return Err(format!("{}", err)),
    };

    let mut buf: [u8; 4096] = [0; 4096];
    let count = syscall::fpath(file.as_raw_fd() as usize, &mut buf)
        .map_err(|err| format!("{}", err))?;

    String::from_utf8(Vec::from(&buf[..count]))
        .map_err(|err| format!("{}", err))
}

//TODO: determine x, y of display by talking to orbital instead of guessing!
pub fn get_display_rects() -> Result<Vec<DisplayRect>, String> {
    let url = get_full_url(
        &env::var("DISPLAY").or(Err("DISPLAY not set"))?
    )?;

    let mut url_parts = url.split(':');
    let scheme_name = url_parts.next().ok_or(format!("no scheme name"))?;
    let path = url_parts.next().ok_or(format!("no path"))?;

    let mut path_parts = path.split('/');
    let vt_screen = path_parts.next().unwrap_or("");
    let width = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);
    let height = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);

    let mut display_rects = vec![DisplayRect {
        x: 0,
        y: 0,
        width,
        height,
    }];

    // If display server supports multiple displays in a VT
    if vt_screen.contains('.') {
        // Look for other screens in the same VT
        let mut parts = vt_screen.split('.');
        let vt_i = parts.next().unwrap_or("").parse::<usize>().unwrap_or(0);
        let start_screen_i = parts.next().unwrap_or("").parse::<usize>().unwrap_or(0);
        //TODO: determine maximum number of screens
        for screen_i in start_screen_i + 1..1024 {
            let url = match get_full_url(&format!("{}:{}.{}", scheme_name, vt_i, screen_i)) {
                Ok(ok) => ok,
                //TODO: only check for ENOENT?
                Err(_err) => break,
            };

            let mut url_parts = url.split(':');
            let _scheme_name = url_parts.next().ok_or(format!("no scheme name"))?;
            let path = url_parts.next().ok_or(format!("no path"))?;

            let mut path_parts = path.split('/');
            let _vt_screen = path_parts.next().unwrap_or("");
            let width = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);
            let height = path_parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);

            let x = if let Some(last) = display_rects.last() {
                last.x + last.width as i32
            } else {
                0
            };

            display_rects.push(DisplayRect {
                x,
                y: 0,
                width,
                height,
            });
        }
    }

    Ok(display_rects)
}
