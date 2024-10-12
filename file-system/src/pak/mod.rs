use std::fs::File;
use zip::ZipArchive;

pub struct Pak {
    file: File,
    archive: ZipArchive<File>,
}
