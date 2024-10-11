use std::io::{self, Cursor, Read, Seek, SeekFrom};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use simd_json::OwnedValue;

const MAGIC: [u8; 4] = [0x11, 0x00, 0x00, 0x00];
const VERSION: usize = 0x00;
const NAME_CRC: usize = 0x04;
const NAME_OFFSET_FROM_STRING: usize = 0x08;
const TYPE_CRC: usize = 0x12;
const TYPE_OFFSET_FROM_STRING: usize = 0x16;
const NUM_COLUMNS: usize = 0x44;
const NUM_ROWS: usize = 0x48;
const HEADER: usize = 0x5c;
const HEADER_BYTE_SIZE: usize = 12;
const CELL_BYTE_SIZE: usize = 8;
const DATA_END: usize = 0x38;

struct Meta<'a> {
    crc: &'a [u8; 4],
    pointer: &'a [u8; 4],
}

struct Header<'a> {
    magic: &'a [u8; 4],
    name: Meta<'a>,
    _type: Meta<'a>,
    field4: &'a [u8; 4],
    strings_size: u64,
    padding: [u8; 24],
    data_size: u32,
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum ColumnType {
    String = 0x01,
    Number = 0x02,
    Boolean = 0x03,
}

#[repr(C, packed)]
struct DataMetadata<'a> {
    output: Meta<'a>,
    num_cols: u32,
    num_rows: u32,
    reserved: u128,
}
#[derive(Debug, Clone, Default)]
pub struct Datasheet {
    pub version: u32,
    pub name: String,
    pub _type: String,
    pub column_count: usize,
    pub row_count: usize,
    header: Vec<HeaderCell>,
    rows: Vec<DatasheetRow>,
}

#[derive(Debug, Clone)]
pub struct HeaderCell {
    text: String,
    _type: u32,
}

pub type DatasheetRow = Vec<DatasheetCell>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasheetCell {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Default, Debug, Clone)]
pub struct Metadata {
    crc32: u32,
    data: [u8; 4],
}

impl Datasheet {
    pub fn meta(&self) -> Value {
        serde_json::json!({
            "type": self._type,
            "name": self.name,
            "fields": self.header.iter().map(|field| {
                let text = &field.text;
                let _type = match &field._type {
                     1 => "string",
                     2 => "number",
                     3 => "boolean",
                     _ => unimplemented!()
                    };
                    (text, _type)
            }).collect::<IndexMap<_, _>>(),
        })
    }

    pub fn to_sql(&self) -> String {
        let create = format!(
            "CREATE TABLE '{}'(\n\t{}\n);\n",
            self.name,
            self.header
                .iter()
                .enumerate()
                .map(|(i, header)| format!(
                    "'{}' {}{}",
                    header.text,
                    match header._type {
                        1 => "TEXT",
                        2 => "REAL",
                        3 => "INT",
                        _ => unreachable!("type not supported"),
                    },
                    match i {
                        0 => " PRIMARY KEY",
                        _ => "",
                    }
                ))
                .collect::<Vec<_>>()
                .join(",\n\t")
        );

        let insert = format!(
            "INSERT INTO '{}' ('{}') VALUES\n\t({});\n",
            self.name,
            self.header
                .iter()
                .map(|header| header.text.to_owned())
                .collect::<Vec<_>>()
                .join("','"),
            self.rows
                .iter()
                .map(|row| row
                    .iter()
                    .map(|cell| match cell {
                        DatasheetCell::String(v) => v.to_owned(),
                        DatasheetCell::Number(v) => v.to_owned().to_string(),
                        DatasheetCell::Boolean(v) => (*v as u32).to_owned().to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(","))
                .collect::<Vec<_>>()
                .join("),\n\t(")
        );

        format!("{}\n\n{}", create, insert)
    }

    pub fn json_value(&self) -> OwnedValue {
        simd_json::json!(self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let value = match cell {
                            DatasheetCell::String(value) => Value::String(value.into()),
                            DatasheetCell::Number(value) => {
                                if value.fract() == 0.0 {
                                    Value::Number((*value as i64).into())
                                } else {
                                    Number::from_f64(*value).into()
                                }
                            }
                            DatasheetCell::Boolean(value) => Value::Bool(*value),
                        };
                        (&self.header[i].text, value)
                    })
                    .collect::<IndexMap<_, _>>()
            })
            .collect::<Vec<_>>())
    }

    pub fn to_json(&self) -> String {
        json!(self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let value = match cell {
                            DatasheetCell::String(value) => Value::String(value.into()),
                            DatasheetCell::Number(value) => {
                                if value.fract() == 0.0 {
                                    Value::Number((*value as i64).into())
                                } else {
                                    Number::from_f64(*value).into()
                                }
                            }
                            DatasheetCell::Boolean(value) => Value::Bool(*value),
                        };
                        (&self.header[i].text, value)
                    })
                    .collect::<IndexMap<_, _>>()
            })
            .collect::<Vec<_>>())
        .to_string()
    }

    pub fn to_json_simd(&self, pretty: bool) -> Result<String, simd_json::Error> {
        let value = &simd_json::json!(self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let value = match cell {
                            DatasheetCell::String(value) => {
                                simd_json::value::owned::Value::String(value.into())
                            }
                            DatasheetCell::Number(value) => {
                                if value.fract() == 0.0 {
                                    simd_json::value::owned::Value::Static((*value as i64).into())
                                } else {
                                    simd_json::value::owned::Value::Static((*value).into())
                                }
                            }
                            DatasheetCell::Boolean(value) => {
                                simd_json::value::owned::Value::Static((*value).into())
                            }
                        };
                        (&self.header[i].text, value)
                    })
                    .collect::<IndexMap<_, _>>()
            })
            .collect::<Vec<_>>());
        if pretty {
            simd_json::to_string_pretty(value)
        } else {
            simd_json::to_string(value)
        }
    }

    pub fn to_csv(&self) -> String {
        let mut csv = String::new();

        // Write the header row
        for (i, header) in self.header.iter().enumerate() {
            if i > 0 {
                csv.push(',');
            }
            csv.push_str(&(header.text.clone()));
        }
        csv.push('\n');

        // Write the data rows
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i > 0 {
                    csv.push(',');
                }
                match cell {
                    DatasheetCell::String(value) => {
                        csv.push_str(value);
                    }
                    DatasheetCell::Number(value) => {
                        if value.fract() == 0.0 {
                            csv.push_str(&(*value as i64).to_string());
                        } else {
                            csv.push_str(&value.to_string());
                        }
                    }
                    DatasheetCell::Boolean(value) => csv.push_str(&value.to_string()),
                }
            }
            csv.push('\n');
        }
        csv
    }

    pub fn to_yaml(&self) -> String {
        let mut rows = Vec::new();
        for row in &self.rows {
            let mut json_row = IndexMap::new();
            for (i, cell) in row.iter().enumerate() {
                match cell {
                    DatasheetCell::String(value) => {
                        json_row.insert(&self.header[i].text, Value::String(value.into()));
                    }
                    DatasheetCell::Number(value) => {
                        if value.fract() == 0.0 {
                            json_row.insert(
                                &self.header[i].text,
                                Value::Number((*value as i64).into()),
                            );
                        } else {
                            json_row.insert(&self.header[i].text, Number::from_f64(*value).into());
                        }
                    }
                    DatasheetCell::Boolean(value) => {
                        json_row.insert(&self.header[i].text, Value::Bool(*value));
                    }
                }
            }
            rows.push(json_row);
        }
        serde_yml::to_string(&rows).unwrap()
    }
}

impl<R: Read> From<&mut R> for Datasheet {
    fn from(value: &mut R) -> Self {
        // value.rewind().unwrap();
        from_reader(value).unwrap()
    }
}

fn from_reader<R: Read>(data: &mut R) -> io::Result<Datasheet> {
    let mut buf = vec![];
    data.read_to_end(&mut buf)?;
    let mut data = Cursor::new(buf);
    data.rewind()?;
    let mut buffer = [0; 4];

    data.read_exact(&mut buffer)?;
    let version = u32::from_le_bytes(buffer);

    // datatable crc  -- i32
    data.seek(SeekFrom::Current(4))?;
    data.read_exact(&mut buffer)?;
    let name_offset = u32::from_le_bytes(buffer);

    data.seek(SeekFrom::Current(4))?;
    data.read_exact(&mut buffer)?;
    let _type_offset = u32::from_le_bytes(buffer);

    data.seek(SeekFrom::Current(36))?;
    data.read_exact(&mut buffer)?;
    let data_end_offset = u32::from_le_bytes(buffer);

    data.read_exact(&mut buffer)?;
    let _ = u32::from_le_bytes(buffer);

    data.seek(SeekFrom::Current(4))?;
    data.read_exact(&mut buffer)?;
    let column_count = u32::from_le_bytes(buffer) as usize;

    data.read_exact(&mut buffer)?;
    let row_count = u32::from_le_bytes(buffer) as usize;

    // SKIP TO HEADER OFFSET
    data.seek(SeekFrom::Current(16))?;

    let strings_offset = data_end_offset as usize + DATA_END + 4;

    let mut header = Vec::with_capacity(column_count);
    for _ in 0..column_count {
        let meta = Metadata {
            crc32: {
                data.read_exact(&mut buffer)?;
                u32::from_le_bytes(buffer)
            },
            data: {
                data.read_exact(&mut buffer)?;
                buffer
            },
        };

        let position = data.stream_position()?;
        let offset = strings_offset + i32::from_le_bytes(meta.data) as usize;
        data.seek(SeekFrom::Start(offset as u64))?;
        let text = read_string(&mut data)?;
        data.seek(SeekFrom::Start(position))?;
        data.read_exact(&mut buffer)?;

        // let mut hasher = Hasher::new();
        // hasher.update(text.as_bytes());
        // let crc = hasher.finalize();
        // dbg!(&text, &meta.crc32, crc);

        let _type = u32::from_le_bytes(buffer);
        header.push(HeaderCell { text, _type });
    }

    let mut rows = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        let mut cells = Vec::with_capacity(column_count);
        for j in 0..column_count {
            let _type = header[j]._type;
            let meta = Metadata {
                crc32: {
                    data.read_exact(&mut buffer)?;
                    u32::from_le_bytes(buffer)
                },
                data: {
                    data.read_exact(&mut buffer)?;
                    buffer
                },
            };
            let value = match _type {
                1 => {
                    let position = data.stream_position()?;
                    let offset = strings_offset + u32::from_le_bytes(meta.data) as usize;
                    data.seek(SeekFrom::Start(offset as u64))?;
                    let string = read_string(&mut data)?;
                    data.seek(SeekFrom::Start(position))?;
                    Ok(DatasheetCell::String(string))
                }
                2 => Ok(DatasheetCell::Number(f32::from_le_bytes(meta.data) as f64)),
                3 => Ok(DatasheetCell::Boolean(i32::from_le_bytes(meta.data) != 0)),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unknown cell type",
                )),
            }?;
            cells.push(value);
        }
        rows.push(cells);
    }

    data.seek(SeekFrom::Start(
        (strings_offset as u32 + name_offset).into(),
    ))?;
    let name = read_string(&mut data)?;
    data.seek(SeekFrom::Start(
        (strings_offset as u32 + _type_offset).into(),
    ))?;
    let _type = read_string(&mut data)?;

    Ok(Datasheet {
        header,
        rows,
        version,
        row_count,
        column_count,
        name,
        _type,
    })
}

fn read_string<R: Read + Seek>(data: &mut R) -> io::Result<String> {
    let mut string = vec![];
    let mut buf = [0u8; 1];

    loop {
        data.read_exact(&mut buf)?;
        if buf[0] == 0 {
            break;
        }
        string.push(buf[0]);
    }
    String::from_utf8(string).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
