//! Open File Description (OFD) locks for Linux.

use std::fs::File;
use std::os::fd::AsRawFd;

pub struct RangeLock(File, i64, i64);

impl RangeLock {
    pub fn init(f: File, start: i64, len: i64, block: bool) -> Option<Self> {
        let flock = libc::flock {
            l_type: libc::F_WRLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: start,
            l_len: len,
            l_pid: 0,
        };

        let fd = f.as_raw_fd();
        let res = unsafe {
            libc::fcntl(
                fd,
                if block {
                    libc::F_OFD_SETLKW
                } else {
                    libc::F_OFD_SETLK
                },
                &flock,
            )
        };
        if res == 0 {
            return Some(Self(f, start, len));
        }
        None
    }
}

impl Drop for RangeLock {
    fn drop(&mut self) {
        let fd = self.0.as_raw_fd();
        let flock = libc::flock {
            l_type: libc::F_UNLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: self.1,
            l_len: self.2,
            l_pid: 0,
        };
        let res = unsafe { libc::fcntl(fd, libc::F_OFD_SETLK, &flock) };
        assert_eq!(0, res);
    }
}

impl core::ops::Deref for RangeLock {
    type Target = File;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
