/// Drawing a PPM into the framebuffer.

use std::io::BufReader;
use std::fs::File;
use std::env;
use fbsplash::FramebufferDevice;
use fbsplash::parse_netbpm_header;

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
        _ = f.sync();
    }
    Ok(())
}
