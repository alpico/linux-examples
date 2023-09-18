//! Count bytes in a directory tree - optimized version.
use al_crunch_pool::{Pool, PoolOptions, Sender};
use std::os::linux::fs::MetadataExt;
use std::path::Path;

/// Recursively visit the directories.
fn visit(
    sender: &Sender<WorkerState>,
    path: &Path,
    state: &mut WorkerState,
) -> std::io::Result<()> {
    // add them for the directories
    let metadata = path.metadata().unwrap();
    state.size += metadata.st_blocks();
    state.count += 1;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;

        // recurse if needed
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            if sender.is_full() {
                let _ = visit(sender, &path, state);
            } else {
                let sender2 = sender.clone();
                sender.send(state, move |state| {
                    let _ = visit(&sender2, &path, state);
                });
            }
        } else {
            state.size += entry.metadata()?.st_blocks();
            state.count += 1;
        }
    }
    Ok(())
}

/// The state to be held by each worker.
#[derive(Default)]
pub struct WorkerState {
    size: u64,
    count: usize,
}

fn main() -> std::io::Result<()> {
    for path in std::env::args().skip(1) {
        let options = PoolOptions::default().one_is_zero().io_bound();
        let pool = Pool::new(options, (), |_| Default::default(), |x: WorkerState| x);

        let mut state = WorkerState::default();
        let sender = pool.sender().clone();
        let pn = path.clone();
        pool.sender().send(&mut state, move |state| {
            let _ = visit(&sender, Path::new(&pn), state);
        });

        // wait until all workers are finished
        for x in pool.join().iter() {
            state.size += x.size;
            state.count += x.count;
        }
        println!("{path} {} {}", state.count, state.size << 9);
    }
    Ok(())
}
