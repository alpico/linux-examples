//! A disk usage implementation that directly uses the libc.

#![feature(rustc_private)]
extern crate libc;

use al_crunch_pool::{Pool, Sender};

/// Visit the directories recursively.
fn visit(sender: &Sender<WorkerState>, file: FileDescriptor, state: &mut WorkerState) {
    let mut buf = [0u8; 4096];
    loop {
        let s = unsafe { libc::syscall(libc::SYS_getdents64, file.0, buf.as_mut_ptr(), buf.len()) };
        if s < 1 {
            assert_eq!(s, 0, "{:?}", std::io::Error::last_os_error());
            break;
        }
        let mut pos = 0;
        let mut more = true;
        while pos < s && more {
            let entry = unsafe { *(buf.as_ptr().offset(pos as isize) as *const libc::dirent64) };
            pos += entry.d_reclen as i64;
            more = entry.d_off != i64::MAX;

            // recurse if needed
            if entry.d_type == libc::DT_DIR
                && !(entry.d_name[0] == b'.' as i8 && entry.d_name[1] == 0)
            {
                // skip the parent directory as well
                if entry.d_name[0] == b'.' as i8
                    && entry.d_name[1] == b'.' as i8
                    && entry.d_name[2] == 0
                {
                    continue;
                }
                let child = FileDescriptor::new(&file, entry.d_name.as_ptr()).unwrap();
                let sender2 = sender.clone();
                sender.send(state, move |state| {
                    visit(&sender2, child, state);
                });
            } else {
                let mut stat = core::mem::MaybeUninit::<libc::stat>::uninit();
                let res = unsafe {
                    libc::fstatat(
                        file.0,
                        entry.d_name.as_ptr(),
                        stat.as_mut_ptr(),
                        libc::AT_SYMLINK_NOFOLLOW,
                    )
                };
                if res != 0 {
                    panic!("{:?}", std::io::Error::last_os_error());
                }
                let stat = unsafe { stat.assume_init() };
                state.blocks += stat.st_blocks as u64;
                state.count += 1;
            }
        }
        if !more {
            break;
        }
    }
}




/// A type to wrap a file-descriptor to close on drop.
struct FileDescriptor(i32);

impl FileDescriptor {
    fn new(parent: &FileDescriptor, path: *const i8) -> Option<Self> {
        let fd = unsafe { libc::openat(parent.0, path, libc::O_DIRECTORY | libc::O_RDONLY, 0) };
        if fd < 0 {
            let name = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap_or("") };
            eprintln!("{name}: {:?}", std::io::Error::last_os_error().kind());
            return None;
        }
        Some(Self(fd))
    }
}
impl Drop for FileDescriptor {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}


#[derive(Default)]
struct WorkerState {
    count: u64,
    blocks: u64,
}

fn main() {
    let curwd = FileDescriptor(libc::AT_FDCWD);
    for path in std::env::args().skip(1) {
        let cpath = std::ffi::CString::new(path.clone()).unwrap();
        let pool = Pool::default();
        
        let state = &mut Default::default();
        let sender = pool.sender().clone();
        let fd = FileDescriptor::new(&curwd, cpath.as_ptr()).unwrap();
        pool.sender().send(state, move |state: &mut WorkerState| {
            visit(&sender, fd, state);
        });

        // aggregate the count of all workers
        for v in pool.join() {
            state.count += v.count;
            state.blocks += v.blocks;
        }
        println!("{} {} {}", path, state.count, state.blocks << 9);
    }
}
