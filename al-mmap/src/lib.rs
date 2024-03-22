//! A read-only memory mapped object.

use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom},
};

#[derive(Clone)]
pub struct Mmap<'a>(pub &'a [u8]);

impl<'a> Mmap<'a> {
    ///  Memory-mapping the file.  A length of zero means entire file.
    pub fn new(
        filename: &str,
        direct: bool,
        mut length: u64,
        offset: i64,
    ) -> Result<Self, std::io::Error> {
        let mut fd = OpenOptions::new()
            .read(true)
            .custom_flags(if direct { libc::O_DIRECT } else { 0 })
            .open(filename)?;

        if length == 0 {
            length = fd.seek(SeekFrom::End(0))?;
        }

        // get a memory mapping
        let ptr = unsafe {
            libc::mmap(
                0 as _,
                length as _,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                offset,
            )
        };
        assert_ne!(libc::MAP_FAILED, ptr);
        let data = unsafe { core::slice::from_raw_parts(ptr as _, length as _) };

        // tell the kernel to no read-ahead
        let x = unsafe { libc::madvise(ptr, length as _, libc::MADV_RANDOM) };
        assert_eq!(x, 0);
        Ok(Self(data))
    }
}

impl Drop for Mmap<'_> {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.0.as_ptr() as *mut libc::c_void, self.0.len()) };
        self.0 = &[];
    }
}
