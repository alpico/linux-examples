//! The sender object.

use crate::channel;

/// The Sender type.
///
/// Wraps a crossbeam::channel::Sender and some worker state to be able to implement `send` on it.
pub struct Sender<W>(pub(crate) channel::Sender<SenderFunction<W>>);

impl<W> Sender<W> {
    /// Send the message.
    ///
    /// The job is executed synchronously if the bounded channel is full.
    pub fn send<F>(&self, worker: &mut W, job: F)
    where
        F: FnOnce(&mut W) + Send + 'static,
    {
        // execute the jobs - this also handles the zero-slots case
        if self.0.is_full() {
            job(worker);
        } else {
            match self.0.try_send(Box::new(job)) {
                Ok(()) => {}
                // there is a race condition on the previous check - thus execute it here instead
                Err(channel::TrySendError::Full(job)) => job(worker),
                Err(e) => {
                    panic!("{}", e)
                }
            };
        }
    }

    /// Return the full indiction of the underlying queue.
    ///
    /// This may be used to optimize the sending code-path.
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }
}

impl<W> Clone for Sender<W> {
    /// Clone the underlying crossbeam::channel::Sender.
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// The sender function.
pub(crate) type SenderFunction<W> = Box<dyn FnOnce(&mut W) + Send>;
