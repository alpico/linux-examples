//! List a directory tree.
use al_crunch_pool::{PoolOptions, Sender};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// Recursively visit the directories.
fn visit(
    sender: &Sender<WorkerState>,
    path: &Path,
    worker: &mut WorkerState,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        // output the current path
        worker.writer.write_all(path.as_os_str().as_bytes())?;
        worker.writer.write_all(b"\n")?;

        // recurse into dirs
        if entry.file_type()?.is_dir() {
            let sender2 = sender.clone();
            sender.send(worker, move |state| {
                let _ = visit(&sender2, &path, state);
            });
        }
    }
    Ok(())
}

/// The state to be held by the worker.
pub struct WorkerState {
    writer: Box<dyn Write>,
}

impl Default for WorkerState {
    fn default() -> Self {
        Self {
            writer: Box::new(std::io::BufWriter::with_capacity(
                16 << 10,
                std::io::stdout(),
            )),
        }
    }
}

fn main() -> std::io::Result<()> {
    let pool = PoolOptions::default().one_is_zero().io_bound().build();
    for path in std::env::args().skip(1) {
        // output the path to be compatible with find(1)
        println!("{path}");
        let sender = pool.sender().clone();
        pool.sender().send(&mut Default::default(), move |state| {
            let _ = visit(&sender, Path::new(&path), state);
        });
    }

    pool.join();
    Ok(())
}
