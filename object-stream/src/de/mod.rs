// use crate::{
//     error::Error, ST_BINARYFLAG_HAS_NAME, ST_BINARYFLAG_HAS_VALUE, ST_BINARYFLAG_HAS_VERSION,
// };
// use serde::de;
// use std::{fmt, io::Read};

// pub struct Deserializer<R> {
//     reader: R,
//     version: u32,
// }

// impl<R: Read> Deserializer<R> {
//     pub fn new(reader: R, version: u32) -> Self {
//         Deserializer { reader, version }
//     }
// }

// impl<'de, 'a, R: Read> de::Deserializer<'de> for &'a mut Deserializer<R> {
//     type Error = Error;

//     fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         let mut buf = [0; 1];
//         self.reader.read_exact(&mut buf).map_err(Error::Io)?;
//         visitor.visit_u8(buf[0])
//     }

//     fn deserialize_struct<V>(
//         self,
//         _name: &'static str,
//         _fields: &'static [&'static str],
//         visitor: V,
//     ) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         visitor.visit_map(StructAccess::new(self)?)
//     }

//     fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         let mut buf = [0; 4];
//         self.reader.read_exact(&mut buf).map_err(Error::Io)?;
//         visitor.visit_u32(u32::from_be_bytes(buf))
//     }

//     fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         let mut buf = [0; 4];
//         self.reader.read_exact(&mut buf).map_err(Error::Io)?;
//         let len = u32::from_be_bytes(buf) as usize;
//         let mut buf = vec![0; len];
//         self.reader.read_exact(&mut buf).map_err(Error::Io)?;
//         visitor.visit_bytes(&buf)
//     }

//     fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_unit_struct<V>(
//         self,
//         name: &'static str,
//         visitor: V,
//     ) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_newtype_struct<V>(
//         self,
//         name: &'static str,
//         visitor: V,
//     ) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_tuple_struct<V>(
//         self,
//         name: &'static str,
//         len: usize,
//         visitor: V,
//     ) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_enum<V>(
//         self,
//         name: &'static str,
//         variants: &'static [&'static str],
//         visitor: V,
//     ) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::Visitor<'de>,
//     {
//         todo!()
//     }

//     // Implement other methods...
// }

// struct StructAccess<'a, R: 'a> {
//     de: &'a mut Deserializer<R>,
//     flags: u8,
// }

// impl<'a, R: Read> StructAccess<'a, R> {
//     fn new(de: &'a mut Deserializer<R>) -> Result<Self, Error> {
//         let mut buf = [0; 1];
//         de.reader.read_exact(&mut buf).map_err(Error::Io)?;
//         let flags = buf[0];
//         Ok(StructAccess { de, flags })
//     }
// }

// impl<'de, 'a, R: Read> de::MapAccess<'de> for StructAccess<'a, R> {
//     type Error = Error;

//     fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
//     where
//         K: de::DeserializeSeed<'de>,
//     {
//         // Use an enum to represent the fields
//         enum Field {
//             Flags,
//             NameCrc,
//             Version,
//             Id,
//             Specialization,
//             DataSize,
//             Data,
//             Elements,
//         }

//         // Implement a visitor for the field names
//         struct FieldVisitor;

//         impl<'de> de::Visitor<'de> for FieldVisitor {
//             type Value = Field;

//             fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//                 formatter.write_str("a field name")
//             }

//             fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
//             where
//                 E: de::Error,
//             {
//                 match value {
//                     "flags" => Ok(Field::Flags),
//                     "name_crc" => Ok(Field::NameCrc),
//                     "version" => Ok(Field::Version),
//                     "id" => Ok(Field::Id),
//                     "specialization" => Ok(Field::Specialization),
//                     "data_size" => Ok(Field::DataSize),
//                     "data" => Ok(Field::Data),
//                     "elements" => Ok(Field::Elements),
//                     _ => Err(E::unknown_field(value, FIELDS)),
//                 }
//             }
//         }

//         // Deserialize the field name
//         let field: Field = seed.deserialize(FieldVisitor)?;

//         // Determine which field to return next based on flags and previous fields
//         match field {
//             Field::Flags => Ok(Some(Field::Flags)),
//             Field::NameCrc if self.flags & ST_BINARYFLAG_HAS_NAME > 0 => Ok(Some(Field::NameCrc)),
//             Field::Version if self.flags & ST_BINARYFLAG_HAS_VERSION > 0 => {
//                 Ok(Some(Field::Version))
//             }
//             Field::Id => Ok(Some(Field::Id)),
//             Field::Specialization if self.de.version == 2 => Ok(Some(Field::Specialization)),
//             Field::DataSize if self.flags & ST_BINARYFLAG_HAS_VALUE > 0 => {
//                 Ok(Some(Field::DataSize))
//             }
//             Field::Data if self.flags & ST_BINARYFLAG_HAS_VALUE > 0 => Ok(Some(Field::Data)),
//             Field::Elements => Ok(Some(Field::Elements)),
//             _ => Ok(None),
//         }
//     }

//     fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
//     where
//         V: de::DeserializeSeed<'de>,
//     {
//         seed.deserialize(&mut *self.de)
//     }
// }

// // Define the list of fields for error reporting
// const FIELDS: &[&str] = &[
//     "flags",
//     "name_crc",
//     "version",
//     "id",
//     "specialization",
//     "data_size",
//     "data",
//     "elements",
// ];
