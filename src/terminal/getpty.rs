use std::os::unix::io::RawFd;
use std::path::PathBuf;

#[cfg(target_os="linux")]
extern crate libc;

#[cfg(target_os="linux")]
pub fn getpty() -> (RawFd, PathBuf) {
    use libc::{c_char, c_int, c_ulong};
    use std::ffi::CStr;
    use std::fs::OpenOptions;
    use std::io::Error;

    const TIOCPKT: c_ulong = 0x5420;
    extern "C" {
        fn ptsname(fd: c_int) -> *const c_char;
        fn grantpt(fd: c_int) -> c_int;
        fn unlockpt(fd: c_int) -> c_int;
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }

    let master_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/ptmx")
        .unwrap()
        .into_raw_fd();
    unsafe {
        let mut flag: c_int = 1;
        if ioctl(master_fd, TIOCPKT, &mut flag as *mut c_int) < 0 {
            panic!("ioctl: {:?}", Error::last_os_error());
        }
        grantpt(master_fd);
        unlockpt(master_fd);
    }

    let tty_path = unsafe { PathBuf::from(CStr::from_ptr(ptsname(master_fd)).to_string_lossy().into_owned()) };
    (master_fd, tty_path)
}

#[cfg(target_os="redox")]
pub fn getpty() -> (RawFd, PathBuf) {
    use syscall;

    let master = syscall::open("pty:", syscall::O_RDWR | syscall::O_CREAT | syscall::O_NONBLOCK).unwrap();
    let mut buf: [u8; 4096] = [0; 4096];
    let count = syscall::fpath(master, &mut buf).unwrap();
    (master, PathBuf::from(unsafe { String::from_utf8_unchecked(Vec::from(&buf[..count])) }))
}
