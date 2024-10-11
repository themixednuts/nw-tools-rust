use std::io;

pub trait IArgs<'a> {
    type Value: 'a;
    fn configure(&mut self, value: Self::Value) -> io::Result<()>;
}

pub trait IDatabase {
    fn load(&self, conn: &rusqlite::Connection) -> rusqlite::Result<Option<String>>;
    fn save(&self, conn: &rusqlite::Connection) -> rusqlite::Result<usize>;
}
