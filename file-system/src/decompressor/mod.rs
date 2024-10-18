use crate::{
    azcs::{self, is_azcs},
    FileType, FILESYSTEM,
};
use cli::{
    commands::Commands,
    common::{
        datasheet::DatasheetFormat, dds::DDSFormat, distribution::DistributionFormat,
        objectstream::ObjectStreamFormat, vshapec::VShapeFormat,
    },
    ARGS,
};
use dashmap::DashMap;
use datasheet::Datasheet;
use flate2::Decompress;
use image_dds::ImageFormat;
use luac_parser::*;
use object_stream::{from_reader, JSONObjectStream, XMLObjectStream};
use quick_xml::se::Serializer;
use rayon::prelude::*;
use serde::Serialize;
use std::io::{self, Cursor, Read, Seek, Write};
use tracing::Instrument;
use vshapec;
use zip::{read::ZipFile, CompressionMethod};

#[derive()]
pub struct Decompressor<'a, 'b> {
    localization: Option<&'a DashMap<String, Option<String>>>,
    zip: &'a mut ZipFile<'b>,
    buf: Vec<u8>,
}

impl<'a, 'b> Decompressor<'a, 'b> {
    /// Creates a new [`Decompressor`].
    pub fn try_new(
        zip: &'a mut ZipFile<'b>,
        localization: Option<&'a DashMap<String, Option<String>>>,
    ) -> io::Result<Self> {
        let size = zip.size() as usize;
        let mut value = Self {
            localization,
            zip,
            buf: Vec::with_capacity(size),
        };
        value.decompress()?;
        Ok(value)
    }
    // pub fn with_buf(
    //     zip: &'a mut ZipFile<'b>,
    //     localization: &'a Option<DashMap<String, Option<String>>>,
    //     buf: &mut R,
    // ) -> Self {
    //     let size = zip.size() as usize;
    //     Self {
    //         localization,
    //         zip,
    //         buf,
    //     }
    // }
    pub fn decompress(&mut self) -> io::Result<()> {
        if self.zip.size() == 0 {
            return Ok(());
        }

        match self.zip.compression() {
            CompressionMethod::Stored => std::io::copy(&mut self.zip, &mut self.buf),
            CompressionMethod::Deflated => {
                let mut bytes = [0; 2];
                self.zip.read_exact(&mut bytes)?;
                if [0x78, 0xda] == bytes {
                    let mut zip = flate2::read::ZlibDecoder::new_with_decompress(
                        Cursor::new(bytes).chain(&mut self.zip),
                        Decompress::new(true),
                    );
                    std::io::copy(&mut zip, &mut self.buf)
                } else {
                    let mut zip =
                        flate2::read::DeflateDecoder::new(Cursor::new(bytes).chain(&mut self.zip));
                    std::io::copy(&mut zip, &mut self.buf)
                }
            }
            #[allow(deprecated)]
            CompressionMethod::Unsupported(15) => {
                let mut compressed = vec![];
                std::io::copy(self.zip, &mut compressed)?;
                self.buf.resize(self.zip.size() as usize, 0);

                oodle_safe::decompress(
                    &compressed,
                    &mut self.buf,
                    None,
                    None,
                    None,
                    Some(oodle_safe::DecodeThreadPhase::All),
                )
                .map(|size| size as u64)
                .map_err(|_| io::Error::other(format!("Error with oodle_safe::decompress.",)))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                "CompressionMethod not supported",
            )),
        }?;

        let mut sig = self.buf[..4].try_into().unwrap();
        if is_azcs(&mut sig) {
            let mut tmp = Vec::with_capacity(self.zip.size() as usize);
            {
                let mut slice = &mut self.buf.as_slice();
                let mut reader = azcs::decompress(&mut slice).unwrap();
                std::io::copy(&mut reader, &mut tmp)?;
            }
            self.buf = tmp;
        };

        Ok(())
    }

    pub fn size(&mut self) {}

    pub fn compressed_size(&mut self) {}

    pub fn file_type(&self) -> io::Result<FileType> {
        let _type = match (self.buf.as_slice(), self.zip.name()) {
            ([0x04, 0x00, 0x1B, 0x4C, 0x75, ..], _) => match &ARGS.command {
                Commands::Extract(cmd) => FileType::Luac(cmd.luac),
                _ => unreachable!(),
            },
            ([0x00, 0x00, 0x00, 0x00, 0x03, ..], _) => match &ARGS.command {
                Commands::Extract(extract) => {
                    FileType::ObjectStream(&extract.objectstream.objectstream)
                }
                _ => unreachable!(),
            },
            ([0x11, 0x00, 0x00, 0x00, ..], _) => match &ARGS.command {
                Commands::Extract(extract) => FileType::Datasheet(&extract.datasheet.datasheet),
                _ => unreachable!(),
            },
            (_, n) if n.ends_with(".distribution") => match &ARGS.command {
                Commands::Extract(cmd) => FileType::Distribution(&cmd.distribution.distribution),
                _ => unreachable!(),
            },
            (_, n) if n.ends_with(".vshapec") => match &ARGS.command {
                Commands::Extract(cmd) => FileType::VShapeC(&cmd.vshapec.vshapec),
                _ => unreachable!(),
            },
            (_, n) if n.ends_with(".dds") => match &ARGS.command {
                Commands::Extract(cmd) => FileType::DDS(&cmd.dds.dds),
                _ => unreachable!(),
            },
            _ => FileType::default(),
        };

        Ok(_type)
    }

    pub fn to_writer<W: Write>(&self, writer: &'_ mut W) -> io::Result<Option<Metadata<'_>>> {
        let file_type = self.file_type()?;
        let mut extra = None;

        let _size = match &file_type {
            FileType::Luac(b) => {
                let mut buf = &self.buf[2..];
                match b {
                    true => {
                        // let mut byte_code = luac_parser::parse(buf).unwrap();

                        // let msg_pack = byte_code.to_msgpack().unwrap();
                        // let mut pack = msg_pack.as_slice();
                        std::io::copy(&mut buf, writer)
                    }
                    false => std::io::copy(&mut buf, writer),
                }
            }
            FileType::DDS(fmt) => match fmt {
                DDSFormat::BYTES => std::io::copy(&mut self.buf.as_slice(), writer),
                DDSFormat::PNG => {
                    let fs = FILESYSTEM.get().unwrap();
                    let mut files = fs
                        .files(Some(&format!("{}.*", self.zip.name())))
                        .into_iter()
                        .collect::<Vec<_>>();

                    files.sort_unstable_by(|(s, _), (s2, _)| {
                        let s_is_a = s.to_str().map_or(false, |name| name.ends_with(".a"));
                        let s2_is_a = s2.to_str().map_or(false, |name| name.ends_with(".a"));

                        match (s_is_a, s2_is_a) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => {
                                natord::compare(s.to_str().expect("msg"), s2.to_str().expect("msg"))
                            }
                        }
                    });

                    let mut buf = self.buf.clone();

                    buf.extend(
                        files
                            .into_par_iter()
                            .map(|(p, _)| fs.open(p).unwrap())
                            .flatten()
                            .collect::<Vec<u8>>()
                            .into_iter(),
                    );

                    let mut buf = Cursor::new(buf);
                    let dds = ddsfile::Dds::read(&mut buf).unwrap();
                    let mut image = image_dds::image_from_dds(&dds, 0).unwrap();

                    let mut buf = Cursor::new(Vec::with_capacity(image.len()));
                    image.write_to(&mut buf, image::ImageFormat::Png);
                    buf.set_position(0);
                    std::io::copy(&mut buf, writer)
                }
                DDSFormat::JPEG => {
                    let fs = FILESYSTEM.get().unwrap();
                    let mut files = fs
                        .files(Some(&format!("{}.*", self.zip.name())))
                        .into_iter()
                        .collect::<Vec<_>>();

                    files.sort_unstable_by(|(s, _), (s2, _)| {
                        let s_is_a = s.to_str().map_or(false, |name| name.ends_with(".a"));
                        let s2_is_a = s2.to_str().map_or(false, |name| name.ends_with(".a"));

                        match (s_is_a, s2_is_a) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => {
                                natord::compare(s.to_str().expect("msg"), s2.to_str().expect("msg"))
                            }
                        }
                    });

                    let mut buf = self.buf.clone();

                    buf.extend(
                        files
                            .into_par_iter()
                            .map(|(p, _)| fs.open(p).unwrap())
                            .flatten()
                            .collect::<Vec<u8>>()
                            .into_iter(),
                    );

                    let mut buf = Cursor::new(buf);
                    let dds = ddsfile::Dds::read(&mut buf).unwrap();
                    let mut image = image_dds::image_from_dds(&dds, 0).unwrap();

                    let mut buf = Cursor::new(Vec::with_capacity(image.len()));
                    image.write_to(&mut buf, image::ImageFormat::Jpeg);
                    buf.set_position(0);
                    std::io::copy(&mut buf, writer)
                }
                DDSFormat::WEBP => {
                    let fs = FILESYSTEM.get().unwrap();
                    let mut files = fs
                        .files(Some(&format!("{}.*", self.zip.name())))
                        .into_iter()
                        .collect::<Vec<_>>();

                    files.sort_unstable_by(|(s, _), (s2, _)| {
                        let s_is_a = s.to_str().map_or(false, |name| name.ends_with(".a"));
                        let s2_is_a = s2.to_str().map_or(false, |name| name.ends_with(".a"));

                        match (s_is_a, s2_is_a) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => {
                                natord::compare(s.to_str().expect("msg"), s2.to_str().expect("msg"))
                            }
                        }
                    });

                    let mut buf = self.buf.clone();

                    buf.extend(
                        files
                            .into_par_iter()
                            .map(|(p, _)| fs.open(p).unwrap())
                            .flatten()
                            .collect::<Vec<u8>>()
                            .into_iter(),
                    );

                    let mut buf = Cursor::new(buf);
                    let dds = ddsfile::Dds::read(&mut buf).unwrap();
                    let mut image = image_dds::image_from_dds(&dds, 0).unwrap();

                    let mut buf = Cursor::new(Vec::with_capacity(image.len()));
                    image.write_to(&mut buf, image::ImageFormat::WebP);
                    buf.set_position(0);
                    std::io::copy(&mut buf, writer)
                }
                DDSFormat::FLAT => {
                    let fs = FILESYSTEM.get().unwrap();
                    let mut files = fs
                        .files(Some(&format!("{}.*", self.zip.name())))
                        .into_iter()
                        .collect::<Vec<_>>();

                    files.sort_unstable_by(|(s, _), (s2, _)| {
                        let s_is_a = s.to_str().map_or(false, |name| name.ends_with(".a"));
                        let s2_is_a = s2.to_str().map_or(false, |name| name.ends_with(".a"));

                        match (s_is_a, s2_is_a) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => {
                                natord::compare(s.to_str().expect("msg"), s2.to_str().expect("msg"))
                            }
                        }
                        // natord::compare(s.to_str().expect("msg"), s2.to_str().expect("msg"))
                    });
                    files.reverse();

                    let mut buf = self.buf.clone();

                    buf.extend(
                        files
                            .into_par_iter()
                            .map(|(p, _)| fs.open(p).unwrap())
                            .flatten()
                            .collect::<Vec<u8>>()
                            .into_iter(),
                    );

                    let mut buf = Cursor::new(buf);
                    std::io::copy(&mut buf, writer)
                }
            },
            FileType::Distribution(fmt) => match fmt {
                DistributionFormat::MINI => {
                    let dist =
                        distribution::Distribution::from_reader(&mut self.buf.as_slice()).unwrap();
                    let mut buf = serde_json::to_vec(&dist).unwrap();
                    std::io::copy(&mut buf.as_slice(), writer)
                }
                DistributionFormat::PRETTY => {
                    let dist =
                        distribution::Distribution::from_reader(&mut self.buf.as_slice()).unwrap();
                    let mut buf = serde_json::to_vec_pretty(&dist).unwrap();

                    std::io::copy(&mut buf.as_slice(), writer)
                }
                DistributionFormat::YAML => {
                    let dist =
                        distribution::Distribution::from_reader(&mut self.buf.as_slice()).unwrap();
                    let mut buf = serde_yml::to_string(&dist).unwrap();

                    std::io::copy(&mut buf.as_bytes(), writer)
                }
                _ => std::io::copy(&mut self.buf.as_slice(), writer),
            },
            FileType::VShapeC(fmt) => match fmt {
                VShapeFormat::MINI => {
                    let vshape = vshapec::VShapeC::from_reader(self.buf.as_slice())?;
                    let mut buf = serde_json::to_vec(&vshape).unwrap();
                    std::io::copy(&mut buf.as_slice(), writer)
                }
                VShapeFormat::PRETTY => {
                    let vshape = vshapec::VShapeC::from_reader(self.buf.as_slice())?;
                    let mut buf = serde_json::to_vec_pretty(&vshape).unwrap();

                    std::io::copy(&mut buf.as_slice(), writer)
                }
                VShapeFormat::YAML => {
                    let vshape = vshapec::VShapeC::from_reader(self.buf.as_slice())?;
                    let mut buf = serde_yml::to_string(&vshape).unwrap();

                    std::io::copy(&mut buf.as_bytes(), writer)
                }
                _ => std::io::copy(&mut self.buf.as_slice(), writer),
            },
            FileType::ObjectStream(fmt) => {
                // early return no serialziation
                if **fmt == ObjectStreamFormat::BYTES {
                    std::io::copy(&mut self.buf.as_slice(), writer)?;
                    return Ok(None);
                };
                let hashes = FILESYSTEM.get().map(|fs| &fs.hashes);
                let Ok(obj_stream) = from_reader(&mut self.buf.as_slice(), hashes) else {
                    std::io::copy(&mut self.buf.as_slice(), writer)?;
                    return Ok(None);
                };
                match fmt {
                    ObjectStreamFormat::XML => {
                        let obj_stream = XMLObjectStream::from(obj_stream);
                        let mut buf = String::new();
                        let mut ser = Serializer::new(&mut buf);
                        ser.indent('\t', 2);
                        obj_stream.serialize(ser).unwrap();
                        std::io::copy(&mut buf.as_bytes(), writer)
                    }
                    ObjectStreamFormat::MINI => {
                        let obj_stream = JSONObjectStream::from(obj_stream);
                        let string = serde_json::to_string(&obj_stream)
                            .expect("couldnt parse object stream to json");
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    ObjectStreamFormat::PRETTY => {
                        let obj_stream = JSONObjectStream::from(obj_stream);
                        let string = serde_json::to_string_pretty(&obj_stream)
                            .expect("couldnt parse object stream to json");
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    _ => std::io::copy(&mut self.buf.as_slice(), writer),
                }
            }
            FileType::Datasheet(fmt) => {
                let mut datasheet = Datasheet::try_from(self.buf.to_owned()).unwrap();

                datasheet.with_localization(self.localization);

                // if **fmt == DatasheetFormat::BYTES {
                //     return Ok((
                //         std::io::copy(&mut sig.chain(reader), writer)?,
                //         file_type,
                //         Some(Metadata::Datasheet(datasheet.to_owned())),
                //     ));
                // };

                extra = Some(Metadata::Datasheet(datasheet.to_owned()));

                // dbg!(&fmt);
                match fmt {
                    DatasheetFormat::MINI => {
                        let string = serde_json::to_string(&datasheet.to_json())?;
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    DatasheetFormat::PRETTY => {
                        let string = serde_json::to_string_pretty(&datasheet.to_json())?;
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    DatasheetFormat::YAML => {
                        let string = datasheet.to_yaml();
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    DatasheetFormat::CSV => {
                        let string = datasheet.to_csv();
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                    DatasheetFormat::BYTES => std::io::copy(&mut self.buf.as_slice(), writer),
                    DatasheetFormat::XML => todo!(),
                    DatasheetFormat::SQL => {
                        let string = datasheet.to_sql();
                        std::io::copy(&mut string.as_bytes(), writer)
                    }
                }
            }
            _ => std::io::copy(&mut self.buf.as_slice(), writer),
        }?;

        Ok(extra)
    }
}

pub enum Metadata<'a> {
    Datasheet(Datasheet<'a>),
}
