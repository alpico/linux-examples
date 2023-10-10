/// Drawing a PPM into the framebuffer.
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::fd::AsRawFd;
extern crate libc;

#[derive(Debug, Default)]
#[repr(C)]
struct FBVarScreenInfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xofs: u32,
    yofs: u32,
    bpp: u32,
    _y: u32,
    _z: [u64; 16],
}

#[derive(Debug, Default)]
#[repr(C)]
struct FBFixScreenInfo {
    id: [u8; 16],
    _x: [u8; 30],
    line_len: usize,
    _y: [u8; 10],
}

#[derive(Debug)]
struct FramebufferDevice<'a> {
    var: FBVarScreenInfo,
    fix: FBFixScreenInfo,
    slice: &'a mut [u8],
}

impl FramebufferDevice<'_> {
    fn new(filename: &str) -> Result<Self, std::io::Error> {
        let file = File::options().read(true).write(true).open(filename)?;
        let mut var: FBVarScreenInfo = Default::default();
        let mut fix: FBFixScreenInfo = Default::default();
        unsafe {
            if libc::ioctl(file.as_raw_fd(), 0x4600, &mut var) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(file.as_raw_fd(), 0x4602, &mut fix) != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        let slice = unsafe {
            let len = fix.line_len * var.yres_virtual as usize;
            let ptr = libc::mmap(
                core::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            );
            if ptr == libc::MAP_FAILED {
                return Err(std::io::Error::last_os_error());
            }

            core::slice::from_raw_parts_mut(ptr as *mut u8, len)
        };

        Ok(Self { var, fix, slice })
    }

    /// Convert from bgr to rgba
    fn convert(dst: &mut [u8], src: &[u8]) {
        for o in 0..3 {
            for i in 0..src.len() / 3 {
                dst[i * 4 + o] = src[i * 3 + 2 - o];
            }
        }
    }

    /// Bitblt a raw RBG image from the reader.
    pub fn bitblt(
        &mut self,
        reader: &mut dyn BufRead,
        width: usize,
        height: usize,
        ofsx: isize,
    ) -> Option<()> {
        // bytes required per output pixel
        let bytes = self.var.bpp as usize / 8;

        // the line in the image
        let mut line = vec![0; width * 3];

        // position the image in the center
        let posy = (std::cmp::max(self.var.yres as usize, height) - height) / 2;
        let posx = (std::cmp::max(self.var.xres as usize, width) - width) / 2 + ofsx as usize;
        for i in 0..self.var.yres as usize {
            let ofs =
                self.fix.line_len * (i + self.var.yofs as usize) + self.var.xofs as usize * bytes;
            let output = &mut self.slice[ofs..ofs + self.fix.line_len];
            if i >= posy && i < posy + height {
                // inside the picture
                output[..posx * bytes].fill(0);
                reader.read_exact(&mut line).unwrap();
                Self::convert(&mut output[posx * bytes..], &line);
                output[(posx + width) * bytes..].fill(0);
            } else {
                output.fill(0);
            }
        }
        Some(())
    }

    // Synchronize the memory mapping with the physical framebuffer.
    pub fn sync(&mut self) -> Result<(), std::io::Error> {
        if 0 == unsafe {
            libc::msync(
                self.slice.as_mut_ptr() as *mut libc::c_void,
                self.slice.len(),
                libc::MS_SYNC,
            )
        } {
            return Ok(());
        }
        Err(std::io::Error::last_os_error())
    }
}

impl Drop for FramebufferDevice<'_> {
    // Unmap the memory mapping.
    fn drop(&mut self) {
        _ = unsafe {
            libc::munmap(
                self.slice.as_mut_ptr() as *mut libc::c_void,
                self.slice.len(),
            )
        };
    }
}

/// This parses all P{1-6} headers.
fn parse_netbpm_header(reader: &mut dyn BufRead) -> Option<(String, usize, usize, usize)> {
    let mut state = [String::new(), String::new(), String::new(), String::new()];
    let mut pos = 0;
    while pos < 4 && state[pos].len() < 8 {
        let mut ch = [0u8];
        reader.read_exact(&mut ch).ok()?;
        match ch[0] {
            b'#' => {
                // comment - strip until newline
                let _ = reader.read_line(&mut String::new());
            }
            x if x.is_ascii_whitespace() => {
                // new token
                if !state[pos].is_empty() {
                    pos += 1;
                }
            }
            _ =>
            // some char
            {
                state[pos].push_str(std::str::from_utf8(&ch).ok()?)
            }
        }
    }
    Some((
        state[0].clone(),
        state[1].parse::<usize>().ok()?,
        state[2].parse::<usize>().ok()?,
        state[3].parse::<usize>().ok()?,
    ))
}

fn main() -> Result<(), std::io::Error> {
    let mut f = FramebufferDevice::new("/dev/fb0")?;
    for frame in 0..256 {
        for filename in env::args().skip(1) {
            let file = File::open(filename.clone())?;
            let mut reader = BufReader::new(file);
            let (magic, width, height, depth) = parse_netbpm_header(&mut reader).unwrap();
            if frame == 0 {
                println!("{filename} {magic} {width}x{height}-{depth}");
            }
            f.bitblt(&mut reader, width, height, frame);
        }
        f.sync()?;
    }
    Ok(())
}
