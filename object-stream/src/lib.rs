use flate2::bufread::ZlibDecoder;
use pak::reader::ReadExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    io::{self, Cursor, Read, Seek, Take},
};
use uuid::Uuid;

const AZCS_SIGNATURE: &'static [u8; 4] = b"AZCS";
const BINARY_STREAM_TAG: u8 = 0;
const XML_STREAM_TAG: u8 = b'<';
const JSON_STREAM_TAG: u8 = b'{';

#[repr(u8)]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum StreamTag {
    #[default]
    Binary = BINARY_STREAM_TAG,
    Xml = XML_STREAM_TAG,
    Json = JSON_STREAM_TAG,
}

impl TryFrom<u8> for StreamTag {
    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0 => Ok(Self::Binary),
            b'<' => Ok(Self::Xml),
            b'{' => Ok(Self::Json),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid stream tag value"),
            )),
        }
    }

    type Error = io::Error;
}

impl Into<u8> for StreamTag {
    fn into(self) -> u8 {
        match self {
            Self::Binary => BINARY_STREAM_TAG,
            Self::Json => JSON_STREAM_TAG,
            Self::Xml => XML_STREAM_TAG,
        }
    }
}

const ST_BINARYFLAG_MASK: u8 = 0xF8;
const ST_BINARY_VALUE_SIZE_MASK: u8 = 0x07;
const ST_BINARYFLAG_ELEMENT_HEADER: u8 = 1 << 3;
const ST_BINARYFLAG_HAS_VALUE: u8 = 1 << 4;
const ST_BINARYFLAG_EXTRA_SIZE_FIELD: u8 = 1 << 5;
const ST_BINARYFLAG_HAS_NAME: u8 = 1 << 6;
const ST_BINARYFLAG_HAS_VERSION: u8 = 1 << 7;
const ST_BINARYFLAG_ELEMENT_END: u8 = 0;

const UNCOMPRESSED_SIGNATURES: [[u8; 5]; 3] = [
    [0x00, 0x00, 0x00, 0x00, 0x03],
    [0x00, 0x00, 0x00, 0x00, 0x02],
    [0x00, 0x00, 0x00, 0x00, 0x01],
];

#[derive(Default, Debug)]
pub struct ObjectStream {
    tag: StreamTag,
    version: u32,
    elements: Vec<Element>,
}

impl ObjectStream {
    pub fn query_elements<F>(&self, query: F) -> Option<&Element>
    where
        F: Fn(&Element) -> bool,
    {
        for element in &self.elements {
            if let Some(result) = element.query_elements(&query) {
                return Some(result);
            };
        }
        None
    }

    pub fn to_json(
        &self,
        uuids: &HashMap<String, String>,
        crcs: &HashMap<String, String>,
    ) -> String {
        serde_json::to_string_pretty(&json!({
            "name": "ObjectStream",
            "version": self.version,
            "Objects": self.elements.iter().map(|ele| ele.to_json(&uuids,  &crcs )).collect::<Vec<_>>()
        })).unwrap()
    }
}

#[derive(Default, Debug)]
pub struct Element {
    id: Option<String>,
    name_crc: Option<u32>,
    version: Option<u8>,
    _type: String,
    specialized_type: Option<String>,
    data_size: Option<u32>,
    data: Vec<u8>,
    elements: Vec<Element>,
}

impl Element {
    pub fn query_elements<F>(&self, query: &F) -> Option<&Element>
    where
        F: Fn(&Element) -> bool,
    {
        if query(self) {
            return Some(self);
        };

        for child in &self.elements {
            if let Some(result) = child.query_elements(query) {
                return Some(result);
            }
        }
        None
    }

    pub fn to_json(
        &self,
        uuids: &HashMap<String, String>,
        crcs: &HashMap<String, String>,
    ) -> serde_json::Value {
        let mut json = json!({});
        if let Some(crc) = self.name_crc {
            if let Some(field) = crcs.get(&crc.to_string()) {
                json["field"] = json!(field);
            } else {
                json["field"] = json!(crc.to_string());
            }
        }

        if let Some(type_name) = uuids.get(&self._type) {
            json["typeName"] = json!(type_name);
        }
        json["typeId"] = json!(self._type);

        if let Some(specialized_type) = &self.specialized_type {
            json["specializationTypeId"] = json!(specialized_type);
        }
        if let Some(version) = self.version {
            json["version"] = json!(version);
        }

        let objects = self
            .elements
            .iter()
            .map(|ele| ele.to_json(&uuids, &crcs))
            .collect::<Vec<_>>();

        if objects.len() > 0 {
            json["Objects"] = json!(objects);
        } else {
            match json["typeName"].as_str() {
                Some("int") | Some("short") | Some("char") | Some("AZ::s64") => {
                    match self.data.len() {
                        1 => {
                            json["value"] = json!(i8::from_be_bytes(
                                self.data
                                    .as_slice()
                                    .try_into()
                                    .expect("Vec should be 1 bytes")
                            ));
                        }
                        2 => {
                            json["value"] = json!(i16::from_be_bytes(
                                self.data
                                    .as_slice()
                                    .try_into()
                                    .expect("Vec should be 2 bytes")
                            ));
                        }
                        4 => {
                            json["value"] = json!(i32::from_be_bytes(
                                self.data
                                    .as_slice()
                                    .try_into()
                                    .expect("Vec should be 4 bytes")
                            ));
                        }
                        8 => {
                            json["value"] = json!(i64::from_be_bytes(
                                self.data
                                    .as_slice()
                                    .try_into()
                                    .expect("Vec should be 8 bytes")
                            ));
                        }
                        _ => {}
                    }
                }
                Some("unsigned char")
                | Some("unsigned short")
                | Some("unsigned int")
                | Some("unsigned long")
                | Some("AZ::u64") => match self.data.len() {
                    1 => {
                        json["value"] = json!(u8::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 1 bytes")
                        ));
                    }
                    2 => {
                        json["value"] = json!(u16::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 2 bytes")
                        ));
                    }
                    4 => {
                        json["value"] = json!(u32::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 4 bytes")
                        ));
                    }
                    8 => {
                        json["value"] = json!(u64::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 8 bytes")
                        ));
                    }
                    _ => {}
                },
                Some("float") | Some("double") => match self.data.len() {
                    4 => {
                        json["value"] = json!(f32::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 1 bytes")
                        ));
                    }
                    8 => {
                        json["value"] = json!(f64::from_be_bytes(
                            self.data
                                .as_slice()
                                .try_into()
                                .expect("Vec should be 2 bytes")
                        ));
                    }
                    _ => {}
                },
                Some("bool") => {
                    if self
                        .data_size
                        .is_some_and(|size| size == 1 && self.data.len() == 1)
                    {
                        json["value"] = json!(self.data.as_slice()[0] != 0);
                    }
                }
                Some("Color") => {
                    if self
                        .data_size
                        .is_some_and(|size| size == 16 && self.data.len() == 16)
                    {
                        json["value"] = json!(self
                            .data
                            .chunks(4)
                            .map(|chunk| {
                                let arr: [u8; 4] = chunk.try_into().expect("Should be 4 bytes");
                                f32::from_be_bytes(arr)
                            })
                            .collect::<Vec<_>>());
                    }
                }
                Some("Asset") => {
                    let id = &self.data[0..16];
                    let subid = &self.data[16..32];
                    let _type = &self.data[32..48];
                    let hint_size =
                        usize::from_be_bytes(self.data[48..56].try_into().expect("not usize"));
                    dbg!(&id, &subid, &_type, &hint_size);
                    let hint = String::from_utf8_lossy(&self.data[56..hint_size + 56]);

                    json["value"] = json!(format!(
                        "id={}:{},type={},hint={{{}}}",
                        uuid::Uuid::from_slice(id)
                            .expect("")
                            .braced()
                            .to_string()
                            .to_uppercase(),
                        uuid::Uuid::from_slice(subid)
                            .expect("")
                            .braced()
                            .to_string()
                            .to_uppercase(),
                        uuid::Uuid::from_slice(_type)
                            .expect("")
                            .braced()
                            .to_string()
                            .to_uppercase(),
                        hint
                    ));
                }
                _ => {
                    if self
                        .data_size
                        .is_some_and(|size| self.data.len() == size as usize)
                    {
                        json["value"] = json!(String::from_utf8_lossy(self.data.as_slice()));
                    }
                    // println!("Didnt find typeName?: {:?}", json["typeName"]);
                }
            };
        }

        json
    }
}

#[derive(Debug)]
struct EOE;

impl std::fmt::Display for EOE {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "End of Elements")
    }
}

impl std::error::Error for EOE {}

#[derive(Default, Debug)]
pub struct Header {
    signature: [u8; 4],
    compressor_id: u32,
    uncompressed_size: u64,
}

#[derive(Debug)]
struct CrcMap {
    uuids: HashMap<String, String>,
    crcs: HashMap<String, String>,
}

pub fn parser(reader: &mut Cursor<Vec<u8>>) -> io::Result<ObjectStream> {
    reader.rewind()?;

    let tag = reader.read_u8()?.try_into()?;

    if tag != StreamTag::Binary {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a valid binary stream",
        ));
    }

    let version = reader.read_u32()?;
    let mut stream = ObjectStream {
        tag,
        version,
        elements: vec![],
    };

    loop {
        match read_element(reader, &stream) {
            Ok(element) => stream.elements.push(element),
            Err(e) if e.is::<EOE>() => return Ok(stream),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("{e}"))),
        }
    }
}

pub fn decompress<R: Read + Unpin>(
    reader: &mut R,
    header: &Header,
) -> io::Result<ZlibDecoder<Take<Cursor<Vec<u8>>>>> {
    match header.compressor_id {
        0x73887d3a => handle_zlib(reader),
        0x72fd505e => Err(io::Error::new(
            io::ErrorKind::Other,
            "zstd is not implemented",
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("unsupported compressor_id: 0x{:08x}", header.compressor_id),
        )),
    }
}

pub fn is_azcs<R: Read>(reader: &mut R, buf: &mut [u8; 5]) -> io::Result<Header> {
    match (is_compressed(buf) || is_uncompressed(buf)) && buf.starts_with(AZCS_SIGNATURE) {
        true => {
            let mut header_data = Header::default();
            header_data.signature = *AZCS_SIGNATURE;

            let mut buffer = [0; 4];
            buffer[0] = buf[4];
            reader.read_exact(&mut buffer[1..])?;

            header_data.compressor_id = u32::from_be_bytes(buffer);
            header_data.uncompressed_size = reader.read_u64()?;
            Ok(header_data)
        }
        false => Err(io::Error::new(
            io::ErrorKind::Other,
            "Not an AZCS file stream",
        )),
    }
}

fn read_element(
    reader: &mut Cursor<Vec<u8>>,
    stream: &ObjectStream,
) -> Result<Element, Box<dyn std::error::Error>> {
    let mut element = Element::default();
    let flags = reader.read_u8()?;

    if flags == ST_BINARYFLAG_ELEMENT_END {
        return Err(Box::new(EOE));
    }

    if flags & ST_BINARYFLAG_HAS_NAME > 0 {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        let name_crc = u32::from_be_bytes(buf);
        element.name_crc = Some(name_crc);
    }

    if flags & ST_BINARYFLAG_HAS_VERSION > 0 {
        let mut buffer = [0u8; 1];
        reader.read_exact(&mut buffer)?;
        let version = u8::from_le_bytes(buffer);
        element.version = Some(version);
    }

    let type_data = reader.read_bytes(16)?;
    element._type = Uuid::from_slice(&type_data)?
        .braced()
        .to_string()
        .to_uppercase();

    if stream.version == 2 {
        let specialized_type_data = reader.read_bytes(16)?;
        element.specialized_type = Some(
            Uuid::from_slice(&specialized_type_data)?
                .braced()
                .to_string()
                .to_uppercase(),
        );
    }

    if flags & ST_BINARYFLAG_HAS_VALUE > 0 {
        let value_bytes = flags & ST_BINARY_VALUE_SIZE_MASK;
        element.data_size = if flags & ST_BINARYFLAG_EXTRA_SIZE_FIELD > 0 {
            match value_bytes {
                1 => {
                    let mut buf = [0; 1];
                    reader.read_exact(&mut buf)?;
                    Some(u8::from_le_bytes(buf) as u32)
                }
                2 => {
                    let mut buf = [0; 2];
                    reader.read_exact(&mut buf)?;
                    Some(u16::from_be_bytes(buf) as u32)
                }
                4 => {
                    let mut buf = [0; 4];
                    reader.read_exact(&mut buf)?;
                    Some(u32::from_be_bytes(buf))
                }
                _ => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "Unsupported DataSize Value Byte",
                    )))
                }
            }
        } else {
            Some(value_bytes as u32)
        };
    }

    if let Some(data_size) = element.data_size {
        element.data = reader.read_bytes(data_size as usize)?;
    }

    loop {
        match read_element(reader, stream) {
            Ok(child_element) => element.elements.push(child_element),
            Err(e) if e.is::<EOE>() => return Ok(element),
            Err(e) => return Err(e),
        }
    }
}

pub fn handle_zlib<R: Read + Unpin>(
    reader: &mut R,
) -> io::Result<ZlibDecoder<Take<Cursor<Vec<u8>>>>> {
    let num_seek_points = reader.read_u32()?;
    let num_seek_points_size = num_seek_points * 16;

    let mut compressed = vec![];
    reader.read_to_end(&mut compressed)?;

    // Calculate the number of bytes to read for seek points
    let data_len = compressed.len();
    if data_len < num_seek_points_size as usize {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Invalid compressed data size",
        ));
    }

    let data_without_seek_points_len = data_len - num_seek_points_size as usize;

    // Create a cursor over the relevant portion of the data
    let data_cursor = Cursor::new(compressed).take(data_without_seek_points_len as u64);
    let zr = ZlibDecoder::new(data_cursor);

    Ok(zr)
}

fn crc32(str: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(str);
    hasher.finalize()
}

pub fn is_uncompressed(data: &[u8]) -> bool {
    for &uncompressed_signature in UNCOMPRESSED_SIGNATURES.iter() {
        if data.len() >= uncompressed_signature.len() && data.starts_with(&uncompressed_signature) {
            return true;
        }
    }
    false
}

pub fn is_compressed(data: &[u8]) -> bool {
    if data.len() < AZCS_SIGNATURE.len() {
        return false;
    }

    data.starts_with(AZCS_SIGNATURE)
}

#[cfg(test)]
mod tests {

    use std::{fs::File, io::Write};

    use crc32fast::Hasher;

    use super::*;

    #[test]
    fn test_parser() -> io::Result<()> {
        // let playerattributes_bytes = include_bytes!("../playerbaseattributes.pbadb");
        // let mut cursor = Cursor::new(playerattributes_bytes.to_vec());
        // let object_stream = parser(&mut cursor)?;

        // let first_stream = &object_stream.elements[0].elements[0];
        // if let Some(crc) = &first_stream.name_crc {
        //     println!("name_crc {:08x}", crc);
        // }
        // dbg!(
        //     &first_stream.element_type,
        //     &first_stream.specialized_type,
        //     &first_stream.version,
        //     &first_stream.data,
        // );

        Ok(())
    }

    #[test]
    fn test() -> io::Result<()> {
        let uuids_json = include_bytes!("../../uuids.json");
        let uuids: HashMap<String, String> = serde_json::from_slice(uuids_json).unwrap();
        let crc_json = include_bytes!("../../crcs.json");
        let crcs: HashMap<String, String> = serde_json::from_slice(crc_json).unwrap();

        // let playerattributes_bytes =
        //     include_bytes!("../../file-system/resources/playerbaseattributes.pbadb");
        let playerattributes_bytes = include_bytes!(
            "E:/Extract/NW Live/sharedassets/genericassets/playerbaseattributes.pbadb"
        );
        let mut cursor = Cursor::new(playerattributes_bytes.to_vec());
        let object_stream = parser(&mut cursor)?;
        File::create("test.json")
            .unwrap()
            .write_all(object_stream.to_json(&uuids, &crcs).as_bytes())
            .unwrap();
        // object_stream.to_json(&uuids, &crcs);

        // let element =
        //     object_stream.query_elements(|ele| ele.name_crc.is_some_and(|crc| crc == crc32));

        // dbg!(element);
        let first_stream = &object_stream.elements[0].elements[0];
        dbg!(
            &first_stream.id,
            &first_stream.version,
            &first_stream.name_crc,
            &first_stream._type
        );
        // if let Some(crc) = &first_stream.name_crc {
        //     println!("name_crc {}", crc);
        //     // dbg!(types.iter().find(|&&w| &w[0].2 == crc));
        // }

        Ok(())
    }
}
