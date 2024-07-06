use tokio::sync::broadcast::{channel, Receiver, Sender};
pub struct EventBus<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T: Clone + Send> EventBus<T> {
    pub fn new() -> Self {
        let (sender, receiver) = channel(1000);
        Self { sender, receiver }
    }

    pub fn broadcast(&self, event: T) {}
}
