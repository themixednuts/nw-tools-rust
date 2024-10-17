use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct EventBus {
    pub sender: Arc<Sender<Event>>,
    pub receiver: Receiver<Event>,
}

impl Default for EventBus {
    fn default() -> Self {
        let (tx, rx) = channel(1000);
        Self {
            sender: Arc::new(tx),
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
pub enum Event {
    Error(ErrorType),
    State(StateType),
    Task(TaskType),
}

#[derive(Clone, Debug)]
pub enum ErrorType {}
#[derive(Clone, Debug)]
pub enum TaskType {}
#[derive(Clone, Debug)]
pub enum StateType {}
