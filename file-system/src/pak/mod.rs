use std::io::Cursor;
use zip::ZipArchive;

pub struct Pak<'a> {
    inner: &'a mut ZipArchive<Cursor<&'a [u8]>>,
}
