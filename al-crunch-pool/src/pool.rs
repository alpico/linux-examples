//! Thread pool.

use crate::{channel, sender, PoolOptions, Sender};

/// The thread pool that executes the jobs.
///
/// The generic type W specificies the worker-state.
pub struct Pool<W, X> {
    /// References to the worker threads.
    worker: Vec<std::thread::JoinHandle<X>>,
    /// The sender for putting in the jobs.
    sender: Sender<W>,
}

impl<W, X> Pool<W, X> {
    /// Join all threads.
    pub fn join(self) -> Vec<X> {
        // drop the sender so that we finish if nobody is live anymore
        drop(self.sender);
        self.worker.into_iter().map(|w| w.join().unwrap()).collect()
    }

    /// Return a reference to the sender so that the queue can be filled.
    pub fn sender(&self) -> &Sender<W> {
        &self.sender
    }
}

impl<W: 'static, X: Send + 'static> Pool<W, X> {
    /// Create a new pool.
    ///
    /// - zero threads means that all requests will be executed synchronously
    /// - the `func` maps the param into the worker state
    pub fn new<T: Send + Clone + 'static>(
        options: PoolOptions,
        param: T,
        create: fn(T) -> W,
        destroy: fn(W) -> X,
    ) -> Self {
        let threads = options.get_threads();

        // the bounded Job queue
        let (sender, receiver) =
            channel::bounded::<sender::SenderFunction<W>>(threads * options.slots);

        // creating all worker threads with their own state
        let worker = (0..threads)
            .map(|_| {
                let receiver = receiver.clone();
                let param = param.clone();
                std::thread::spawn(move || {
                    let mut state = create(param);
                    for job in receiver {
                        job(&mut state);
                    }
                    destroy(state)
                })
            })
            .collect();

        Self {
            worker,
            sender: Sender(sender),
        }
    }
}

/// Convinience function.
impl<W: Default + Send + 'static> Default for Pool<W, W> {
    fn default() -> Self {
        crate::Pool::new(PoolOptions::default(), (), |_| Default::default(), |x| x)
    }
}
