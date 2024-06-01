use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::sync::Arc;

const OFFSET_NUM_COLUMNS: usize = 0x44;
const OFFSET_NUM_ROWS: usize = 0x48;
const OFFSET_HEADER: usize = 0x5c;
const OFFSET_HEADER_SIZE_IN_BYTES: usize = 12;
const OFFSET_CELL_SIZE_IN_BYTES: usize = 8;

#[derive(Debug, Clone)]
pub struct HeaderCell {
    text: String,
    _type: i32,
}

pub type DatasheetRow = Vec<DatasheetCell>;

#[derive(Debug, Clone)]
pub enum DatasheetCell {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct Datasheet {
    header: Vec<HeaderCell>,
    rows: Vec<DatasheetRow>,
}

pub fn parse_datasheet<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
) -> io::Result<Datasheet> {
    let column_count = read_i32_le(data, OFFSET_NUM_COLUMNS)? as usize;
    let row_count = read_i32_le(data, OFFSET_NUM_ROWS)? as usize;

    let cells_offset = OFFSET_HEADER + column_count * OFFSET_HEADER_SIZE_IN_BYTES;
    let row_size_in_bytes = OFFSET_CELL_SIZE_IN_BYTES * column_count;
    let strings_offset = cells_offset + row_count * column_count * OFFSET_CELL_SIZE_IN_BYTES;

    let mut header = Vec::with_capacity(column_count);
    for i in 0..column_count {
        let offset = OFFSET_HEADER + i * OFFSET_HEADER_SIZE_IN_BYTES;
        let meta = read_cell_meta(data, offset)?;
        let buffer = meta.data.as_ref(); // Borrow the Vec<u8> from the Arc
        let mut cursor = Cursor::new(&buffer); // Create a Cursor from the Vec<u8>
        let mut buffer = [0; 4];
        cursor.read_exact(&mut buffer)?;

        let _offset = strings_offset + i32::from_le_bytes(buffer) as usize;
        let text = read_string(&mut *data, _offset)?;
        let _type = read_i32_le(data, offset + 8)?;
        header.push(HeaderCell { text, _type });
    }

    let mut rows = Vec::with_capacity(row_count);
    for i in 0..row_count {
        let mut cells = Vec::with_capacity(column_count);
        for j in 0..column_count {
            let cell_offset = cells_offset + i * row_size_in_bytes + j * OFFSET_CELL_SIZE_IN_BYTES;
            let _type = header[j]._type;
            let meta = read_cell_meta(data, cell_offset)?;
            let value = read_cell(data, strings_offset, _type, meta.data)?;
            cells.push(value);
        }
        rows.push(cells);
    }

    Ok(Datasheet { header, rows })
}
pub fn parse_datasheet_test<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
) -> io::Result<Datasheet> {
    // SKIP TO NUM COLUMNS
    let mut skip = [0; OFFSET_NUM_COLUMNS];
    data.read_exact(&mut skip)?;

    let mut buffer = [0; 4];

    data.read_exact(&mut buffer)?;
    let column_count = i32::from_le_bytes(buffer) as usize;
    data.read_exact(&mut buffer)?;
    let row_count = i32::from_le_bytes(buffer) as usize;

    // SKIP TO HEADER OFFSET
    let mut skip = [0; 16];
    data.read_exact(&mut skip)?;

    let header_size_in_bytes = OFFSET_HEADER_SIZE_IN_BYTES * column_count;
    let strings_offset =
        OFFSET_HEADER + header_size_in_bytes + row_count * column_count * OFFSET_CELL_SIZE_IN_BYTES;

    let mut header = Vec::with_capacity(column_count);
    for _ in 0..column_count {
        data.read_exact(&mut buffer)?;
        let mut meta = CellMeta::default();
        meta.hash = i32::from_le_bytes(buffer);
        data.read_exact(&mut buffer)?;
        meta.data = buffer;

        let offset = strings_offset + i32::from_le_bytes(meta.data) as usize;
        let position = data.stream_position()?;
        let text = read_string(&mut *data, offset)?;
        data.seek(SeekFrom::Start(position))?;
        data.read_exact(&mut buffer)?;
        let _type = i32::from_le_bytes(buffer);
        header.push(HeaderCell { text, _type });
    }

    let mut rows = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        let mut cells = Vec::with_capacity(column_count);
        for j in 0..column_count {
            let _type = header[j]._type;
            let meta = CellMeta {
                hash: {
                    data.read_exact(&mut buffer)?;
                    i32::from_le_bytes(buffer)
                },
                data: {
                    data.read_exact(&mut buffer)?;
                    buffer
                },
            };
            let position = data.stream_position()?;
            let value = read_cell(data, strings_offset, _type, meta.data)?;
            data.seek(SeekFrom::Start(position))?;
            cells.push(value);
        }
        rows.push(cells);
    }

    Ok(Datasheet { header, rows })
}

fn read_i32_le<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
    offset: usize,
) -> io::Result<i32> {
    let mut buffer = [0; 4];
    data.seek(SeekFrom::Start(offset as u64))?;
    data.read_exact(&mut buffer)?;
    Ok(i32::from_le_bytes(buffer))
}

fn read_string<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
    offset: usize,
) -> io::Result<String> {
    let mut length = 0;
    while {
        let mut buf = [0u8; 1];
        data.seek(SeekFrom::Start((offset + length) as u64))?;
        data.read_exact(&mut buf)?;
        buf[0] != 0
    } {
        length += 1;
    }
    let mut string = vec![0; length];
    data.seek(SeekFrom::Start(offset as u64))?;
    data.read_exact(&mut string)?;
    String::from_utf8(string).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_cell<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
    offset: usize,
    _type: i32,
    value: [u8; 4],
) -> io::Result<DatasheetCell> {
    let mut cursor = Cursor::new(&value);
    match _type {
        1 => {
            let string_offset = read_i32_le(&mut cursor, 0)? as usize;
            let string = read_string(data, offset + string_offset)?;
            Ok(DatasheetCell::String(string))
        }
        2 => {
            let num = read_f32_le(&mut cursor, 0)?;
            Ok(DatasheetCell::Number(num as f64))
        }
        3 => {
            let boolean = read_i32_le(&mut cursor, 0)? != 0;
            Ok(DatasheetCell::Boolean(boolean))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unknown cell type",
        )),
    }
}

fn read_cell_meta<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
    offset: usize,
) -> io::Result<CellMeta> {
    let hash = read_i32_le(data, offset)?;
    let mut buffer = [0; 4];
    data.seek(SeekFrom::Start((offset + 4) as u64))?;
    data.read_exact(&mut buffer)?;
    Ok(CellMeta { hash, data: buffer })
}

fn read_f32_le<R: Read + Sync + Send + Unpin + Seek>(
    data: &mut R,
    offset: usize,
) -> io::Result<f32> {
    let mut buffer = [0; 4];
    data.seek(SeekFrom::Start(offset as u64))?;
    data.read_exact(&mut buffer)?;
    Ok(f32::from_le_bytes(buffer))
}

#[derive(Default, Debug)]
pub struct CellMeta {
    hash: i32,
    data: [u8; 4],
}
