use std::io;

pub trait InteractiveArgs {
    fn interactive(&mut self) -> io::Result<()>;
}
