use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct EventBus {
    sender: Arc<Sender<Event>>,
    receiver: Receiver<Event>,
}

impl Default for EventBus {
    fn default() -> Self {
        let (tx, rx) = channel(1000);
        Self {
            sender: tx.into(),
            receiver: rx,
        }
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn broadcast(&self) {}
}

#[derive(Clone, Debug)]
enum Event {
    Error(ErrorType),
    State(StateType),
    Task(TaskType),
}

#[derive(Clone, Debug)]
enum ErrorType {}
#[derive(Clone, Debug)]
enum TaskType {}
#[derive(Clone, Debug)]
enum StateType {}
