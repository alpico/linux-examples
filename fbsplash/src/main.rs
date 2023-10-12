use fbsplash::parse_netbpm_header;
use fbsplash::FramebufferDevice;
use std::env;
use std::fs::File;
/// Drawing a PPM into the framebuffer.
use std::io::BufReader;

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
