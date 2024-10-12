use crate::{
    error::{Error, Result},
    ST_BINARYFLAG_HAS_VALUE, ST_BINARY_VALUE_SIZE_MASK,
};
use serde::{ser, Serialize};
use std::io::Write;

pub struct Serializer<W> {
    writer: W,
    version: Option<u32>,
    flags: Option<u8>,
    capture: bool,
    write_size: bool,
    size: Option<u8>,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Serializer {
            writer,
            version: None,
            flags: None,
            capture: false,
            write_size: false,
            size: None,
        }
    }
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, _v: bool) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i8(self, _v: i8) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i16(self, _v: i16) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i32(self, _v: i32) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i64(self, _v: i64) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> std::result::Result<Self::Ok, Self::Error> {
        if self.capture {
            self.flags = Some(v);
            self.capture = false;
        }
        self.writer.write_all(&[v]).map_err(Error::Io)
    }

    fn serialize_u16(self, _v: u16) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> std::result::Result<Self::Ok, Self::Error> {
        if self.version.is_none() {
            self.version = Some(v);
        }
        self.writer.write_all(&v.to_be_bytes()).map_err(Error::Io)
    }

    fn serialize_u64(self, v: u64) -> std::result::Result<Self::Ok, Self::Error> {
        if self.write_size {
            self.write_size = false;
            return match self.size {
                Some(1) => self.serialize_u8(v as u8),
                Some(2) => self.serialize_u16(v as u16),
                Some(4) => self.serialize_u32(v as u32),
                _ => Err(Error::ExpectedValue),
            };
        }
        self.writer.write_all(&v.to_be_bytes()).map_err(Error::Io)
    }

    fn serialize_f32(self, _v: f32) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, _v: f64) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, v: char) -> std::result::Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let c = v.encode_utf8(&mut buf);
        self.writer.write_all(c.as_bytes()).map_err(Error::Io)
    }

    fn serialize_str(self, v: &str) -> std::result::Result<Self::Ok, Self::Error> {
        self.writer.write_all(v.as_bytes()).map_err(Error::Io)
    }

    fn serialize_bytes(self, v: &[u8]) -> std::result::Result<Self::Ok, Self::Error> {
        self.writer.write_all(v).map_err(Error::Io)
    }

    fn serialize_none(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self)
    }

    fn serialize_unit(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(
        self,
        _name: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> std::result::Result<Self::Ok, Self::Error> {
        // let f = format!("name: {} idx: {} variant: {}", name, variant_index, variant);
        // dbg!("IM HERE SERIALIZING A UNIT VARIANT", f);
        // Ok(())
        todo!()
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> std::result::Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn serialize_seq(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(
        self,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(self)
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> std::result::Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStruct, Self::Error> {
        self.flags = None;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::result::Result<Self::SerializeStructVariant, Self::Error> {
        Ok(self)
    }
}

impl<'a, W: Write> ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        match key {
            "flags" => {
                self.capture = true;
            }
            "data_size" => {
                self.write_size = true;
                if let Some(flags) = self.flags {
                    if flags & ST_BINARYFLAG_HAS_VALUE > 0 {
                        let size = flags & ST_BINARY_VALUE_SIZE_MASK;
                        match size {
                            1 => self.size = Some(1),
                            2 => self.size = Some(2),
                            4 => self.size = Some(4),
                            _ => return Ok(()),
                        };
                        return value.serialize(&mut **self);
                    }
                }

                return Ok(());
            }
            "elements" => {
                value.serialize(&mut **self)?;
                self.writer.write_all(&mut [0]).map_err(Error::Io)?;
                return Ok(());
            }
            _ => {}
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn skip_field(&mut self, key: &'static str) -> std::result::Result<(), Self::Error> {
        let _ = key;
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeMap for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _key: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn serialize_value<T>(&mut self, _value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a, W: Write> ser::SerializeSeq for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
impl<'a, W: Write> ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a, W: Write> ser::SerializeTuple for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
impl<'a, W: Write> ser::SerializeTupleStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a, W: Write> ser::SerializeTupleVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> std::result::Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        todo!()
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }
}

pub fn to_writer<T, W>(value: &T, writer: &mut W) -> Result<()>
where
    T: Serialize,
    W: Write,
{
    let mut serializer = Serializer::new(writer);
    value.serialize(&mut serializer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // use std::io::Cursor;

    // use super::*;
    // use crate::{from_reader, ObjectStream};
    // use serde::Serialize;

    #[test]
    fn test_struct() {
        // let byt =
        //     include_bytes!("E:/Extract/nw live/sharedassets/genericassets/fuelcategory.fueldb");

        // let test: ObjectStream = from_reader(&mut Cursor::new(byt)).unwrap();
        // let mut buf = vec![];
        // to_writer(&test, &mut buf).unwrap();
        // dbg!(&test);
        // let mut buf2 = vec![];
        // test.to_writer(&mut buf2).unwrap();
        // assert_eq!(&buf, byt);
        // assert_eq!(&buf2, byt);
    }

    // #[test]
    // fn test_enum() {
    //     #[derive(Serialize)]
    //     enum E {
    //         Unit,
    //         Newtype(u32),
    //         Tuple(u32, u32),
    //         Struct { a: u32 },
    //     }

    //     let u = E::Unit;
    //     let expected = r#""Unit""#;
    //     assert_eq!(to_string(&u).unwrap(), expected);

    //     let n = E::Newtype(1);
    //     let expected = r#"{"Newtype":1}"#;
    //     assert_eq!(to_string(&n).unwrap(), expected);

    //     let t = E::Tuple(1, 2);
    //     let expected = r#"{"Tuple":[1,2]}"#;
    //     assert_eq!(to_string(&t).unwrap(), expected);

    //     let s = E::Struct { a: 1 };
    //     let expected = r#"{"Struct":{"a":1}}"#;
    //     assert_eq!(to_string(&s).unwrap(), expected);
    // }
}
