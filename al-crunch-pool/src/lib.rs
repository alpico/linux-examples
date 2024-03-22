//! A simple multi-threaded pool for executing number-chrunching workloads.

/// Relying on crossbeam to make our life easier.
use crossbeam::channel;

mod options;
pub use options::Options;

mod sender;
pub use sender::Sender;

mod pool;
pub use pool::Pool;

/// Execute the jobs in a scoped pool.
///
/// create: a closure to create per-worker states
/// init: a closure to submit the first jobs.
/// combine: a closure to combine all states into one result
pub fn execute<W, Y: Send, X>(
    options: Options,
    create: impl Fn(usize) -> W + Send + Copy,
    destroy: impl Fn(W) -> Y + Send + Copy,
    init: impl FnOnce(&Sender<W>) -> X,
    combine: impl Fn(X, Y) -> X,
) -> X {
    let threads = options.get_threads();

    // the bounded Job queue
    let (sender, receiver) = channel::bounded::<sender::SenderFunction<W>>(threads * options.slots);

    std::thread::scope(|s| {
        // create the worker threads
        let worker: Vec<std::thread::ScopedJoinHandle<'_, _>> = (0..threads)
            .map(|i| {
                let receiver = receiver.clone();
                s.spawn(move || {
                    let mut state = create(i + 1);
                    for job in receiver {
                        job(&mut state);
                    }

                    destroy(state)
                })
            })
            .collect();

        // submit the initial jobs
        let state = init(&Sender(sender));

        // combine all results
        worker
            .into_iter()
            .map(|w| w.join().unwrap())
            .fold(state, combine)
    })
}
