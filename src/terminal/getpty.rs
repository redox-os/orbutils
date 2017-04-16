use std::os::unix::io::RawFd;

#[cfg(not(target_os="redox"))]
pub fn getpty() -> (RawFd, String) {
    use libc;
    use std::ffi::CStr;
    use std::fs::OpenOptions;
    use std::io::Error;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::io::IntoRawFd;

    extern "C" {
        fn ptsname(fd: libc::c_int) -> *const libc::c_char;
        fn grantpt(fd: libc::c_int) -> libc::c_int;
        fn unlockpt(fd: libc::c_int) -> libc::c_int;
    }

    let master_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/ptmx")
        .unwrap()
        .into_raw_fd();
    unsafe {
        if grantpt(master_fd) < 0 {
            panic!("grantpt: {:?}", Error::last_os_error());
        }
        if unlockpt(master_fd) < 0 {
            panic!("unlockpt: {:?}", Error::last_os_error());
        }
    }

    let tty_path = unsafe { CStr::from_ptr(ptsname(master_fd)).to_string_lossy().into_owned() };
    (master_fd, tty_path)
}

#[cfg(target_os="redox")]
pub fn getpty() -> (RawFd, String) {
    use syscall;

    let master = syscall::open("pty:", syscall::O_RDWR | syscall::O_CREAT | syscall::O_NONBLOCK).unwrap();
    let mut buf: [u8; 4096] = [0; 4096];
    let count = syscall::fpath(master, &mut buf).unwrap();
    (master, unsafe { String::from_utf8_unchecked(Vec::from(&buf[..count])) })
}
