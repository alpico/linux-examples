/// Drawing a PPM into the framebuffer.

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom};
use std::os::fd::AsRawFd;
extern crate libc;

#[derive(Debug, Default)]
#[repr(C)]
struct FBVarScreenInfo {
    xres: u32,
    yres: u32,
    _x: [u32;4],
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
struct FramebufferDevice {
    file: File,
    var: FBVarScreenInfo,
    fix: FBFixScreenInfo,
}

impl FramebufferDevice {
    fn new(filename: &str) -> Option<Self> {
        let file = File::options().write(true).open(filename).ok()?;
        let mut var: FBVarScreenInfo = Default::default();
        let mut fix: FBFixScreenInfo = Default::default();
        unsafe {
            assert_eq!(0, libc::ioctl(file.as_raw_fd(), 0x4600, &mut var));
            assert_eq!(0, libc::ioctl(file.as_raw_fd(), 0x4602, &mut fix));
        }
        Some(Self { file, var, fix } )
    }

    /// Convert from rgb to rgba
    fn convert(dst: &mut [u8], src: &[u8]) {
        for o in 0..3 {
            for i in 0..src.len() / 3 {
                dst[i*4 + o] = src[i*3 + 2 - o]
            }
        }
    }
    
    /// Bitblt a raw RBG image data from the reader.
    fn bitblt(&mut self, reader: &mut dyn BufRead, width: usize, height: usize) ->Option<()> {
        let bytes = self.var.bpp as usize / 8;

        // the output buffer
        let mut buf = vec!();
        buf.resize(self.var.xres as usize * bytes, 0);

        // the line in the image
        let mut line = vec!();
        line.resize(width as usize * 3, 0);
        
        let posy = (std::cmp::max(self.var.yres as usize, height) - height) / 2;
        let posx = (std::cmp::max(self.var.xres as usize, width) - width) / 2;
        for i in 0..self.var.yres as usize {
            let ofs = self.fix.line_len * (i + self.var.yofs as usize) + self.var.xofs as usize * bytes;
            self.file.seek(SeekFrom::Start(ofs as u64)).unwrap();
            if i >= posy && i < posy + height {
                // inside the picture
                reader.read_exact(&mut line).unwrap();
                Self::convert(&mut buf[posx * bytes..], &line);
                self.file.write_all(&buf).unwrap();
            }
            else {
                if i == posy + height {
                    buf.fill(0);
                }
                // outside the picture
                self.file.write_all(&buf).unwrap();
            }
        }
     Some(())   
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
                if state[pos].len() > 0 {
                    pos += 1;
                }
            }
            _ =>
                // some char
                state[pos].push_str(std::str::from_utf8(&ch).ok()?),
        }
    }
    Some((state[0].clone(), state[1].parse::<usize>().ok()?, state[2].parse::<usize>().ok()?, state[3].parse::<usize>().ok()?))
}


fn main() -> Result<(), std::io::Error>{
    let mut f = FramebufferDevice::new("/dev/fb0").unwrap();
    for filename in env::args().skip(1) {
        let file = File::open(filename.clone())?;
        let mut reader = BufReader::new(file);
        let (magic, width, height, depth) = parse_netbpm_header(&mut reader).unwrap();
        println!("{filename} {magic} {width}x{height}-{depth}");
        f.bitblt(&mut reader, width, height);
    }
    Ok(())
}
