pub mod custom_beams {
    use std::time::{Duration, Instant};

    use crossbeam_channel::{bounded, Receiver, SendError, SendTimeoutError, Sender, TrySendError};

    pub struct LooseSender<T> {
        sender: Sender<T>,
        dropper: Receiver<T>,
    }

    #[allow(unused)]
    impl<T> LooseSender<T> {
        pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
            self.sender.try_send(msg)
        }
        pub fn send(&self, msg: T) -> Result<(), SendError<T>> {
            self.sender.send(msg)
        }
        pub fn send_timeout(&self, msg: T, timeout: Duration) -> Result<(), SendTimeoutError<T>> {
            self.sender.send_timeout(msg, timeout)
        }
        pub fn send_deadline(&self, msg: T, deadline: Instant) -> Result<(), SendTimeoutError<T>> {
            self.sender.send_deadline(msg, deadline)
        }
        pub fn is_empty(&self) -> bool {
            self.sender.is_empty()
        }
        pub fn is_full(&self) -> bool {
            self.sender.is_full()
        }
        pub fn len(&self) -> usize {
            self.sender.len()
        }
        pub fn capacity(&self) -> Option<usize> {
            self.sender.capacity()
        }
        pub fn same_channel(&self, other: &Sender<T>) -> bool {
            self.sender.same_channel(other)
        }

        pub fn loosely_send(&self, msg: T) -> Result<(), TrySendError<T>> {
            match self.sender.try_send(msg) {
                Ok(_) => Ok(()),
                Err(TrySendError::Full(data)) => {
                    drop(self.dropper.try_recv());
                    self.sender.try_send(data)
                }
                Err(TrySendError::Disconnected(data)) => Err(TrySendError::Disconnected(data)),
            }
        }
    }

    pub fn loose<T>(cap: usize) -> (LooseSender<T>, Receiver<T>) {
        let (sender, receiver) = bounded(cap);

        let sender = LooseSender {
            sender,
            dropper: receiver.clone(),
        };

        (sender, receiver)
    }
}
