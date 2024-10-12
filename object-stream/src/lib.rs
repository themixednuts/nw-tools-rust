mod de;
mod error;
pub mod ser;
mod types;

use crc32fast::hash;
use serde::{self, Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Cursor, Read, Write};
use utils::{lumberyard::LumberyardSource, types::uuid_data_to_serialize};
use uuid::{self, serde::compact, Uuid};

const ST_BINARYFLAG_MASK: u8 = 0xF8;
const ST_BINARY_VALUE_SIZE_MASK: u8 = 0x07;
const ST_BINARYFLAG_ELEMENT_HEADER: u8 = 1 << 3;
const ST_BINARYFLAG_HAS_VALUE: u8 = 1 << 4;
const ST_BINARYFLAG_EXTRA_SIZE_FIELD: u8 = 1 << 5;
const ST_BINARYFLAG_HAS_NAME: u8 = 1 << 6;
const ST_BINARYFLAG_HAS_VERSION: u8 = 1 << 7;
const ST_BINARYFLAG_ELEMENT_END: u8 = 0;

const BINARY_STREAM_TAG: u8 = 0;
const XML_STREAM_TAG: u8 = b'<';
const JSON_STREAM_TAG: u8 = b'{';

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StreamTag(pub u8);

impl StreamTag {
    pub const BINARY: Self = StreamTag(BINARY_STREAM_TAG);
    pub const XML: Self = StreamTag(XML_STREAM_TAG);
    pub const JSON: Self = StreamTag(JSON_STREAM_TAG);
}

impl Default for StreamTag {
    fn default() -> Self {
        Self::BINARY
    }
}

impl PartialEq<u8> for StreamTag {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ObjectStream {
    _tag: StreamTag,
    version: u32,
    elements: Vec<Element>,
}

impl From<XMLObjectStream> for ObjectStream {
    fn from(value: XMLObjectStream) -> Self {
        Self {
            _tag: StreamTag::BINARY,
            version: value.version,
            elements: value.elements.into_iter().map(Element::from).collect(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "ObjectStream")]
pub struct XMLObjectStream {
    #[serde(rename = "@version")]
    version: u32,
    #[serde(default, rename = "Class")]
    elements: Vec<XMLElement>,
}

impl XMLObjectStream {
    pub fn to_writer(&mut self, buf: &mut impl Write) -> io::Result<u64> {
        let string = quick_xml::se::to_string(self).map_err(std::io::Error::other)?;
        std::io::copy(&mut Cursor::new(string), buf)
    }
}

impl From<ObjectStream> for XMLObjectStream {
    fn from(value: ObjectStream) -> Self {
        Self {
            version: value.version,
            elements: value.elements.into_iter().map(XMLElement::from).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JSONObjectStream {
    name: String,
    version: u32,
    #[serde(rename = "Objects")]
    elements: Vec<JSONElement>,
}

impl JSONObjectStream {
    pub fn to_writer(&self, buf: &mut impl Write) {
        serde_json::to_writer_pretty(buf, self).unwrap()
    }
}
impl From<ObjectStream> for JSONObjectStream {
    fn from(value: ObjectStream) -> Self {
        Self {
            name: "ObjectStream".into(),
            version: value.version,
            elements: value.elements.into_iter().map(JSONElement::from).collect(),
        }
    }
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
}

#[derive(PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct Element {
    flags: u8,
    name_crc: Option<u32>,
    version: Option<u8>,
    #[serde(with = "compact")]
    id: Uuid,
    specialization: Option<Uuid>,
    #[serde(skip)]
    name: String,
    data_size: Option<usize>,
    data: Option<Vec<u8>>,
    elements: Vec<Element>,
    field: Option<String>,
}

impl From<XMLElement> for Element {
    fn from(value: XMLElement) -> Self {
        Self {
            id: value.id,
            name: value.name.to_owned(),
            name_crc: {
                if !value.name.is_empty() {
                    Some(hash(value.name.as_bytes()))
                } else {
                    None
                }
            },
            version: value.version,
            elements: value.elements.into_iter().map(Element::from).collect(),
            ..Default::default()
        }
    }
}

impl From<JSONElement> for Element {
    fn from(value: JSONElement) -> Self {
        Self {
            id: value.id,
            name: value.name.to_owned(),
            // flags: todo!(),
            name_crc: {
                if !value.name.is_empty() {
                    Some(hash(value.name.as_bytes()))
                } else {
                    None
                }
            },
            field: value.field,
            version: value.version,
            // data_size: todo!(),
            // data: todo!(),
            specialization: value.specialization,
            elements: value
                .elements
                .map(|ele| ele.into_iter().map(Element::from).collect())
                .unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct XMLElement {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@field", skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(rename = "@value", skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(rename = "@version", skip_serializing_if = "Option::is_none")]
    version: Option<u8>,
    #[serde(rename = "@type", with = "uuid_braced_uppercase")]
    id: Uuid,
    #[serde(default, rename = "Class")]
    elements: Vec<XMLElement>,
}

impl XMLElement {
    pub fn to_writer(&mut self, buf: &mut (impl Write + std::fmt::Write)) {
        quick_xml::se::to_writer(buf, self).unwrap();
    }
}

impl From<Element> for XMLElement {
    fn from(value: Element) -> Self {
        Self {
            name: value.name,
            field: value.field,
            value: match value.data {
                Some(data) if !data.is_empty() || value.elements.is_empty() => {
                    uuid_data_to_serialize(&value.id, &data, false).ok()
                }
                _ => None,
            },
            version: value.version,
            id: value.id,
            elements: value.elements.into_iter().map(XMLElement::from).collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct JSONElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
    #[serde(rename = "typeId", with = "uuid_braced_uppercase")]
    id: Uuid,
    #[serde(rename = "typeName")]
    name: String,
    #[serde(
        rename = "specializationTypeId",
        with = "option_braced_uppercase",
        skip_serializing_if = "Option::is_none"
    )]
    specialization: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<u8>,
    #[serde(rename = "Objects", skip_serializing_if = "Option::is_none")]
    elements: Option<Vec<JSONElement>>,
}

impl From<Element> for JSONElement {
    fn from(value: Element) -> Self {
        Self {
            field: value.field,
            id: value.id,
            name: value.name,
            specialization: value.specialization,
            value: match &value.data {
                Some(data) if !data.is_empty() => {
                    uuid_data_to_serialize(&value.id, data, true).ok().map(|v| {
                        if !v.is_string() {
                            Value::String(v.to_string())
                        } else {
                            v
                        }
                    })
                }
                Some(data) if data.is_empty() && value.elements.is_empty() => Some("".into()),
                _ => None,
            },
            version: value.version,
            elements: {
                let ele: Vec<JSONElement> =
                    value.elements.into_iter().map(JSONElement::from).collect();
                if ele.is_empty() && value.data.is_some() {
                    None
                } else {
                    Some(ele)
                }
            },
        }
    }
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

    fn to_writer<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&self.flags.to_be_bytes())?;
        if let Some(crc) = self.name_crc {
            writer.write_all(&crc.to_be_bytes())?;
        }
        if let Some(version) = self.version {
            writer.write_all(&version.to_be_bytes())?;
        }
        writer.write_all(&self.id.as_u128().to_be_bytes())?;

        if let Some(specialized) = self.specialization {
            writer.write_all(&specialized.as_u128().to_be_bytes())?;
        }
        if self.flags & ST_BINARYFLAG_HAS_VALUE > 0 {
            let value_bytes = self.flags & ST_BINARY_VALUE_SIZE_MASK;
            if self.flags & ST_BINARYFLAG_EXTRA_SIZE_FIELD > 0 {
                if let Some(size) = self.data_size {
                    match value_bytes {
                        1 => writer.write_all(&(size as u8).to_be_bytes())?,
                        2 => writer.write_all(&(size as u16).to_be_bytes())?,
                        4 => writer.write_all(&(size as u32).to_be_bytes())?,
                        _ => {}
                    };
                }
            };
        };

        if let Some(data) = &self.data {
            writer.write_all(data)?;
        }

        self.elements
            .iter()
            .for_each(|ele| ele.to_writer(writer).unwrap());
        writer.write_all(&[0])?;

        Ok(())
    }
}

pub mod uuid_braced_uppercase {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;
    use uuid_simd::UuidExt;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = uuid.as_braced().to_string().to_uppercase();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse(&s).map_err(serde::de::Error::custom)
    }
}

pub mod option_braced_uppercase {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;
    use uuid_simd::UuidExt;

    pub fn serialize<S>(value: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(uuid) => uuid::serde::braced::serialize(uuid, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<&str>::deserialize(deserializer)?;
        match opt {
            Some(s) => match Uuid::parse(s) {
                Ok(uuid) => Ok(Some(uuid)),
                Err(e) => Err(serde::de::Error::custom(e)),
            },
            None => Ok(None),
        }
    }
}

pub mod option_borrow_braced {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::borrow::Cow;
    use uuid::Uuid;
    use uuid_simd::UuidExt;

    pub fn serialize<S>(value: &Option<Cow<'_, Uuid>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(uuid) => serializer.serialize_str(&uuid.as_braced().to_string().to_uppercase()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Cow<'de, Uuid>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<&str>::deserialize(deserializer)?;
        match opt {
            Some(s) => match Uuid::parse(s) {
                Ok(uuid) => Ok(Some(Cow::Owned(uuid))),
                Err(e) => Err(serde::de::Error::custom(e)),
            },
            None => Ok(None),
        }
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

pub fn from_reader<R>(
    reader: &mut R,
    hashes: Option<&'static LumberyardSource>,
) -> io::Result<ObjectStream>
where
    R: Read,
{
    let mut buf = [0; 1];
    reader.read_exact(&mut buf)?;
    let tag = u8::from_be_bytes(buf);

    if tag != StreamTag::BINARY.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Not valid ObjectStream",
        ));
    }

    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    let version = u32::from_be_bytes(buf);

    let mut stream = ObjectStream {
        _tag: StreamTag::BINARY,
        version,
        elements: vec![],
        ..Default::default()
    };

    loop {
        match read_element(reader, &stream, hashes) {
            Ok(element) => stream.elements.push(element),
            Err(e) if e.is::<EOE>() => return Ok(stream),
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("{e}"))),
        }
    }
}

fn read_element<R>(
    reader: &mut R,
    stream: &ObjectStream,
    hashes: Option<&'static LumberyardSource>,
) -> Result<Element, Box<dyn std::error::Error>>
where
    R: Read,
{
    let mut element = Element::default();
    let mut buf = [0; 16];
    reader.read_exact(&mut buf[..1])?;
    let flags = buf[0];

    if flags == ST_BINARYFLAG_ELEMENT_END {
        return Err(Box::new(EOE));
    }

    if flags & ST_BINARYFLAG_HAS_NAME > 0 {
        reader.read_exact(&mut buf[..4])?;
        let name_crc = u32::from_be_bytes(buf[..4].try_into()?);
        element.field = hashes.and_then(|v| v.crcs.get(&name_crc).cloned());
        element.name_crc = Some(name_crc);
    }

    if flags & ST_BINARYFLAG_HAS_VERSION > 0 {
        reader.read_exact(&mut buf[..1])?;
        let version = buf[0];
        element.version = Some(version);
    }

    reader.read_exact(&mut buf)?;
    element.id = Uuid::from_slice(&buf)?;
    element.name = hashes
        .and_then(|v| v.uuids.get(&element.id).cloned())
        .unwrap_or_default();

    if stream.version == 2 {
        reader.read_exact(&mut buf)?;
        element.specialization = Some(Uuid::from_slice(&buf)?);
    }

    if flags & ST_BINARYFLAG_HAS_VALUE > 0 {
        let value_bytes = flags & ST_BINARY_VALUE_SIZE_MASK;
        element.data_size = if flags & ST_BINARYFLAG_EXTRA_SIZE_FIELD > 0 {
            match value_bytes {
                1 => {
                    reader.read_exact(&mut buf[..1])?;
                    Some(u8::from_le_bytes(buf[..1].try_into()?) as usize)
                }
                2 => {
                    reader.read_exact(&mut buf[..2])?;
                    Some(u16::from_be_bytes(buf[..2].try_into()?) as usize)
                }
                4 => {
                    reader.read_exact(&mut buf[..4])?;
                    Some(u32::from_be_bytes(buf[..4].try_into()?) as usize)
                }
                _ => {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "Unsupported DataSize Value Byte",
                    )))
                }
            }
        } else {
            Some(value_bytes as usize)
        };
    }

    if let Some(data_size) = element.data_size {
        let mut buf = vec![0; data_size];
        reader.read_exact(&mut buf)?;
        element.data = Some(buf);
        if element
            .field
            .as_ref()
            .is_some_and(|v| v == "m_collisionFilterOverride")
            && element.data.as_ref().is_some_and(|v| {
                v != &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                ]
                .to_vec()
            })
        {
            dbg!(&element.data, &element.data_size);
        }
    }
    element.flags = flags;

    loop {
        match read_element(reader, stream, hashes) {
            Ok(child_element) => element.elements.push(child_element),
            Err(e) if e.is::<EOE>() => return Ok(element),
            Err(e) => return Err(e),
        }
    }
}

impl ObjectStream {
    pub fn to_writer<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&0u8.to_be_bytes())?;
        writer.write_all(&self.version.to_be_bytes())?;
        self.elements
            .iter()
            .for_each(|ele| ele.to_writer(writer).unwrap());
        writer.write_all(&[0])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use io::Cursor;
    use quick_xml::DeError;

    use super::*;

    #[test]
    fn binary() -> io::Result<()> {
        // let byt =
        //     include_bytes!("E:/Extract/nw live/sharedassets/genericassets/fuelcategory.fueldb");
        // let object_stream = from_reader(&mut Cursor::new(byt))?;
        // let t: XMLObjectStream = XMLObjectStream::from(object_stream);
        // dbg!(quick_xml::se::to_string(&t).unwrap());

        // dbg!(&object_stream);

        Ok(())
    }

    #[test]
    fn xml() -> Result<(), DeError> {
        let xml = r#"<ObjectStream version="3"><Class name="int" field="test" value="2" type="{72039442-EB38-4D42-A1AD-CB68F7E0EEF6}"/><Class name="Asset" type="{72039442-EB38-4D42-A1AD-CB68F7E0EEF6}"><Class name="int" field="element" value="100" type="{72039442-EB38-4D42-A1AD-CB68F7E0EEF6}"/></Class></ObjectStream>"#;

        let xml_object_stream: XMLObjectStream = quick_xml::de::from_str(&xml)?;
        // dbg!(&xml_object_stream);
        assert_eq!(quick_xml::se::to_string(&xml_object_stream)?.as_str(), xml);
        // let object_stream: BinaryObjectStream = BinaryObjectStream::from(xml_object_stream);
        // dbg!(&object_stream);

        Ok(())
    }

    #[test]
    fn json() -> io::Result<()> {
        let json = r#"{"name":"ObjectStream","version":3,"Objects":[]}"#;
        let json_object_stream: JSONObjectStream = serde_json::from_str(&json)?;
        // dbg!(&json_object_stream);
        assert_eq!(&serde_json::to_string(&json_object_stream)?, &json);
        Ok(())
    }
}

// https://github.com/aws/lumberyard/blob/413ecaf24d7a534801cac64f50272fe3191d278f/dev/Code/Framework/AzCore/AzCore/Serialization/SerializeContext.h#L536
// enum Flags
//           {
//               FLG_POINTER             = (1 << 0),       ///< Element is stored as pointer (it's not a value).
//               FLG_BASE_CLASS          = (1 << 1),       ///< Set if the element is a base class of the holding class.
//               FLG_NO_DEFAULT_VALUE    = (1 << 2),       ///< Set if the class element can't have a default value.
//               FLG_DYNAMIC_FIELD       = (1 << 3),       ///< Set if the class element represents a dynamic field (DynamicSerializableField::m_data).
//               FLG_UI_ELEMENT          = (1 << 4),       ///< Set if the class element represents a UI element tied to the ClassData of its parent.
//           };

// pub fn to_json(&self, w: &mut impl Write) {
//     let value = json!({
//         "name": "ObjectStream",
//         "version": self.version,
//         "Objects": self.elements.iter().map(|ele| ele.to_json()).collect::<Vec<_>>()
//     });

//     serde_json::to_writer_pretty(w, &value).unwrap();
// }
// pub fn to_xml(&self, w: &mut impl Write) {
//     let mut writer = Writer::new_with_indent(w, b'\t', 1);
//     let _w = writer
//         .create_element("ObjectStream")
//         .with_attribute(("version", self.version.to_string().as_str()))
//         .write_inner_content::<_, Error>(|w| {
//             self.elements.iter().for_each(|ele| ele.to_xml(w));
//             Ok(())
//         })
//         .unwrap();
// }

//     pub fn to_xml<T>(&self, writer: &mut Writer<T>)
//     where
//         T: Write,
//     {
//         let fs = FILESYSTEM.get().unwrap();
//         let mut ele = writer.create_element("Class");

//         let mut name = None;
//         if let Some(_type) = fs.uuids.get(self.type_id.as_str()) {
//             name = Some(_type.to_string());
//             let _type = _type.to_string();
//             ele = ele.with_attribute(("name", _type.as_str()));
//         } else {
//             ele = ele.with_attribute(("name", ""));
//         }

//         if let Some(crc) = self.name_crc {
//             if let Some(string) = fs.crcs.get(&crc) {
//                 ele = ele.with_attribute(("field", string.as_str()));
//             } else {
//                 ele = ele.with_attribute(("field", crc.to_string().as_str()));
//             }
//         }

//         if self.data_size.is_some() {
//             ele = match name.as_deref() {
//                 Some("int") | Some("short") | Some("char") | Some("AZ::s64") => {
//                     match self.data.len() {
//                         1 => ele.with_attribute((
//                             "value",
//                             i8::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 1 bytes"),
//                             )
//                             .to_string()
//                             .as_str(),
//                         )),
//                         2 => ele.with_attribute((
//                             "value",
//                             i16::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 1 bytes"),
//                             )
//                             .to_string()
//                             .as_str(),
//                         )),
//                         4 => ele.with_attribute((
//                             "value",
//                             i32::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 1 bytes"),
//                             )
//                             .to_string()
//                             .as_str(),
//                         )),
//                         8 => ele.with_attribute((
//                             "value",
//                             i64::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 1 bytes"),
//                             )
//                             .to_string()
//                             .as_str(),
//                         )),
//                         _ => ele,
//                     }
//                 }
//                 Some("unsigned char")
//                 | Some("unsigned short")
//                 | Some("unsigned int")
//                 | Some("unsigned long")
//                 | Some("AZ::u64") => match self.data.len() {
//                     1 => ele.with_attribute((
//                         "value",
//                         u8::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     2 => ele.with_attribute((
//                         "value",
//                         u16::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     4 => ele.with_attribute((
//                         "value",
//                         u32::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     8 => ele.with_attribute((
//                         "value",
//                         u64::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     _ => ele,
//                 },
//                 Some("float") | Some("double") => match self.data.len() {
//                     4 => ele.with_attribute((
//                         "value",
//                         f32::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     8 => ele.with_attribute((
//                         "value",
//                         f64::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes"),
//                         )
//                         .to_string()
//                         .as_str(),
//                     )),
//                     _ => ele,
//                 },
//                 Some("bool") => {
//                     if self
//                         .data_size
//                         .is_some_and(|size| size == 1 && self.data.len() == 1)
//                     {
//                         ele.with_attribute((
//                             "value",
//                             (self.data.as_slice()[0] != 0).to_string().as_str(),
//                         ))
//                     } else {
//                         ele
//                     }
//                 }
//                 Some("Color") => {
//                     if self
//                         .data_size
//                         .is_some_and(|size| size == 16 && self.data.len() == 16)
//                     {
//                         ele.with_attribute((
//                             "value",
//                             self.data
//                                 .chunks(4)
//                                 .map(|chunk| {
//                                     let arr: [u8; 4] = chunk.try_into().expect("Should be 4 bytes");
//                                     f32::from_be_bytes(arr).to_string()
//                                 })
//                                 .collect::<Vec<_>>()
//                                 .join(" ")
//                                 .as_str(),
//                         ))
//                     } else {
//                         ele
//                     }
//                 }
//                 Some("Asset") => {
//                     let id = &self.data[0..16];
//                     let subid = &self.data[16..32];
//                     let _type = &self.data[32..48];
//                     let hint_size =
//                         usize::from_be_bytes(self.data[48..56].try_into().expect("not usize"));
//                     // dbg!(&id, &subid, &_type, &hint_size);
//                     let hint = String::from_utf8_lossy(&self.data[56..hint_size + 56]);

//                     ele.with_attribute((
//                         "value",
//                         format!(
//                             "id={}:{},type={},hint={{{}}}",
//                             uuid::Uuid::from_slice(id)
//                                 .expect("")
//                                 .braced()
//                                 .to_string()
//                                 .to_uppercase(),
//                             u128::from_le_bytes(subid.try_into().unwrap()),
//                             uuid::Uuid::from_slice(_type)
//                                 .expect("")
//                                 .braced()
//                                 .to_string()
//                                 .to_uppercase(),
//                             hint
//                         )
//                         .as_str(),
//                     ))
//                 }
//                 Some("AZ::Uuid") => ele.with_attribute((
//                     "value",
//                     Uuid::from_slice(&self.data)
//                         .unwrap()
//                         .as_braced()
//                         .to_string()
//                         .to_uppercase()
//                         .as_str(),
//                 )),
//                 Some("Vector3") => {
//                     let x = f32::from_le_bytes(self.data[0..4].try_into().unwrap());
//                     let y = f32::from_le_bytes(self.data[4..8].try_into().unwrap());
//                     let z = f32::from_le_bytes(self.data[8..12].try_into().unwrap());

//                     let formatted_value = format!("{:.7} {:.7} {:.7}", x, y, z);
//                     ele.with_attribute(("value", formatted_value.as_str()))
//                 }
//                 _ => {
//                     match self.data_size {
//                         Some(size) => {
//                             assert_eq!(&(size as usize), &self.data.len());
//                             ele.with_attribute((
//                                 "value",
//                                 String::from_utf8(self.data.to_owned())
//                                     .expect(&format!(
//                                         "Expected utf-8 string; Data Size: {} | CRC: {} | type: {}",
//                                         size,
//                                         self.name_crc.unwrap(),
//                                         self.type_id
//                                     ))
//                                     .to_string()
//                                     .as_str(),
//                             ))
//                         }
//                         None => ele,
//                     }
//                     // println!("Didnt find typeName?: {:?}", json["typeName"]);
//                 }
//             };
//         }

//         if let Some(version) = self.version {
//             ele = ele.with_attribute(("version", version.to_string().as_str()));
//         }
//         ele = ele.with_attribute((
//             "type",
//             self.type_id
//                 .as_braced()
//                 .encode_upper(&mut Uuid::encode_buffer())
//                 .as_ref(),
//         ));

//         if !self.elements.is_empty() {
//             ele.write_inner_content::<_, Error>(|w| {
//                 self.elements.iter().for_each(|ele| ele.to_xml(w));

//                 Ok(())
//             })
//             .unwrap();
//         } else {
//             ele.write_empty().unwrap();
//         }
//     }

//     pub fn to_json(&self) -> serde_json::Value {
//         let fs = FILESYSTEM.get().unwrap();
//         let mut json = json!({});
//         if let Some(crc) = self.name_crc {
//             if let Some(field) = fs.crcs.get(&crc) {
//                 json["field"] = json!(field);
//             } else {
//                 json["field"] = json!(crc.to_string());
//             }
//         }

//         if let Some(type_name) = fs.uuids.get(&self.type_id) {
//             json["typeName"] = json!(type_name);
//         }
//         json["typeId"] = json!(self.type_id);

//         if let Some(specialized_type) = &self.specialization {
//             json["specializationTypeId"] = json!(specialized_type);
//         }
//         if let Some(version) = self.version {
//             json["version"] = json!(version);
//         }

//         let objects = self
//             .elements
//             .iter()
//             .map(|ele| ele.to_json())
//             .collect::<Vec<_>>();

//         if !objects.is_empty() {
//             json["Objects"] = json!(objects);
//         } else {
//             match json["typeName"].as_str() {
//                 Some("int") | Some("short") | Some("char") | Some("AZ::s64") => {
//                     match self.data.len() {
//                         1 => {
//                             json["value"] = json!(i8::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 1 bytes")
//                             ));
//                         }
//                         2 => {
//                             json["value"] = json!(i16::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 2 bytes")
//                             ));
//                         }
//                         4 => {
//                             json["value"] = json!(i32::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 4 bytes")
//                             ));
//                         }
//                         8 => {
//                             json["value"] = json!(i64::from_be_bytes(
//                                 self.data
//                                     .as_slice()
//                                     .try_into()
//                                     .expect("Vec should be 8 bytes")
//                             ));
//                         }
//                         _ => {}
//                     }
//                 }
//                 Some("unsigned char")
//                 | Some("unsigned short")
//                 | Some("unsigned int")
//                 | Some("unsigned long")
//                 | Some("AZ::u64") => match self.data.len() {
//                     1 => {
//                         json["value"] = json!(u8::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes")
//                         ));
//                     }
//                     2 => {
//                         json["value"] = json!(u16::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 2 bytes")
//                         ));
//                     }
//                     4 => {
//                         json["value"] = json!(u32::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 4 bytes")
//                         ));
//                     }
//                     8 => {
//                         json["value"] = json!(u64::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 8 bytes")
//                         ));
//                     }
//                     _ => {}
//                 },
//                 Some("float") | Some("double") => match self.data.len() {
//                     4 => {
//                         json["value"] = json!(f32::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 1 bytes")
//                         ));
//                     }
//                     8 => {
//                         json["value"] = json!(f64::from_be_bytes(
//                             self.data
//                                 .as_slice()
//                                 .try_into()
//                                 .expect("Vec should be 2 bytes")
//                         ));
//                     }
//                     _ => {}
//                 },
//                 Some("bool") => {
//                     if self
//                         .data_size
//                         .is_some_and(|size| size == 1 && self.data.len() == 1)
//                     {
//                         json["value"] = json!(self.data.as_slice()[0] != 0);
//                     }
//                 }
//                 Some("Color") => {
//                     if self
//                         .data_size
//                         .is_some_and(|size| size == 16 && self.data.len() == 16)
//                     {
//                         json["value"] = json!(self
//                             .data
//                             .chunks(4)
//                             .map(|chunk| {
//                                 let arr: [u8; 4] = chunk.try_into().expect("Should be 4 bytes");
//                                 f32::from_be_bytes(arr)
//                             })
//                             .collect::<Vec<_>>());
//                     }
//                 }
//                 Some("Asset") => {
//                     let id = &self.data[0..16];
//                     let subid = &self.data[16..32];
//                     let _type = &self.data[32..48];
//                     let hint_size =
//                         usize::from_be_bytes(self.data[48..56].try_into().expect("not usize"));
//                     // dbg!(&id, &subid, &_type, &hint_size);
//                     let hint = String::from_utf8_lossy(&self.data[56..hint_size + 56]);

//                     json["value"] = json!(format!(
//                         "id={}:{},type={},hint={{{}}}",
//                         uuid::Uuid::from_slice(id)
//                             .expect("")
//                             .braced()
//                             .to_string()
//                             .to_uppercase(),
//                         u128::from_le_bytes(subid.try_into().unwrap()).to_string(),
//                         uuid::Uuid::from_slice(_type)
//                             .expect("")
//                             .braced()
//                             .to_string()
//                             .to_uppercase(),
//                         hint
//                     ));
//                 }
//                 Some("AZ::Uuid") => {
//                     json["value"] = json!(Uuid::from_slice(&self.data)
//                         .unwrap()
//                         .as_braced()
//                         .to_string()
//                         .as_str())
//                 }
//                 _ => {
//                     if self.data_size.is_some_and(|size| self.data.len() == size) {
//                         json["value"] = json!(String::from_utf8_lossy(self.data.as_slice()));
//                     }
//                     // println!("Didnt find typeName?: {:?}", json["typeName"]);
//                 }
//             };
//         }

//         json
//     }

//     fn process_data(&self) -> impl Serialize + Deserialize + ToString {
//         match self.type_id {
//             CHAR => IntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<i8>,
//             },
//             SHORT => IntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<i16>,
//             },
//             INT => IntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<i32>,
//             },
//             LONG => IntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<i64>,
//             },
//             UNSIGNED_CHAR => UIntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<u8>,
//             },
//             UNSIGNED_SHORT => UIntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<u16>,
//             },
//             UNSIGNED_INT => UIntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<u32>,
//             },
//             UNSIGNED_LONG => UIntSerializer {
//                 data: self.data.clone(),
//                 marker: PhantomData::<u64>,
//             },
//             _ => {
//                 unreachable!()
//             }
//         }
//     }
