//! Configuration options for the pool.

/// Collect the config options to make a pool.
#[derive(Clone, Debug)]
pub struct Options {
    threads: Option<usize>,
    pub(crate) slots: usize,
    one_is_zero: bool,
    io_bound: bool,
}

impl Options {
    /// Get the number of threads.
    pub fn get_threads(&self) -> usize {
        match self.threads.unwrap_or(
            std::thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(0),
        ) {
            1 if self.one_is_zero => 0,
            x if self.io_bound => 4 * x,
            x => x,
        }
    }

    /// Set the number of threads.  None means one thread per core.
    pub fn threads(self, threads: Option<usize>) -> Self {
        Self { threads, ..self }
    }

    /// Define the slots per thread.
    pub fn slots(self, slots: usize) -> Self {
        Self { slots, ..self }
    }

    /// Indicate that one in a thread should be mapped to zero to make all calls synchronous.
    pub fn one_is_zero(self) -> Self {
        Self {
            one_is_zero: true,
            ..self
        }
    }

    /// Indicates that the work is depending on external I/O and more threads should be allocated.
    pub fn io_bound(self) -> Self {
        Self {
            io_bound: true,
            ..self
        }
    }

    /// Build the pool directly from the options if the worker state implements Default.
    pub fn build<W: Default + 'static>(self) -> crate::Pool<W, ()> {
        crate::Pool::new(self, (), |_| Default::default(), |_| ())
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            threads: None,
            slots: 8,
            one_is_zero: false,
            io_bound: false,
        }
    }
}
