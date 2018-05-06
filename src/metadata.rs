// Copyright (c) 2018 FaultyRAM
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at
// your option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Resource headers, flags and other metadata.

use std::fmt::{self, Debug, Formatter};
use std::io::Read;
use std::{mem, str};
use Error;

bitflags! {
    #[doc(hidden)]
    pub struct ResourceFlags: u8 {
        /// Indicates that a resource is stored using LZW compression.
        const RDF_LZW = 0x01;
        /// Indicates that a resource is a compound resource.
        const RDF_COMPOUND = 0x02;
        /// Reserved for future use.
        const RDF_RESERVED = 0x04;
        /// Indicates that a resource should be loaded into memory when the resource file
        /// containing it is opened.
        const RDF_LOADONOPEN = 0x08;
        /// Indicates that a resource is on a virtual CD-ROM drive(?).
        const RDF_CDSPOOF = 0x10;
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
/// Resource types.
pub enum ResourceType {
    /// Unknown resource type.
    Unknown = 0,
    /// A string.
    String = 1,
    /// A bitmapped image.
    Image = 2,
    /// A bitmapped font.
    Font = 3,
    /// An animation script.
    AnimationScript = 4,
    /// A palette.
    Palette = 5,
    /// A shading table.
    ShadingTable = 6,
    /// A .voc sound file.
    Voc = 7,
    /// A shape.
    Shape = 8,
    /// A picture.
    Picture = 9,
    /// BABL2 extern records.
    Babl2Extern = 10,
    /// BABL2 relocation records.
    Babl2Reloc = 11,
    /// BABL2 object code.
    Babl2Code = 12,
    /// A BABL2 linked resource header.
    Babl2Header = 13,
    /// Reserved.
    Babl2Reserved = 14,
    /// A 3D object.
    Object3d = 15,
    /// A stencil.
    Stencil = 16,
    /// An LG .mov movie file.
    Movie = 17,
    /// A list of bounding rectangles for images.
    Rectangle = 18,
    /// An application-defined resource type.
    AppDefined0 = 48,
    /// An application-defined resource type.
    AppDefined1 = 49,
    /// An application-defined resource type.
    AppDefined2 = 50,
    /// An application-defined resource type.
    AppDefined3 = 51,
    /// An application-defined resource type.
    AppDefined4 = 52,
    /// An application-defined resource type.
    AppDefined5 = 53,
    /// An application-defined resource type.
    AppDefined6 = 54,
    /// An application-defined resource type.
    AppDefined7 = 55,
    /// An application-defined resource type.
    AppDefined8 = 56,
    /// An application-defined resource type.
    AppDefined9 = 57,
    /// An application-defined resource type.
    AppDefined10 = 58,
    /// An application-defined resource type.
    AppDefined11 = 59,
    /// An application-defined resource type.
    AppDefined12 = 60,
    /// An application-defined resource type.
    AppDefined13 = 61,
    /// An application-defined resource type.
    AppDefined14 = 62,
    /// An application-defined resource type.
    AppDefined15 = 63,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
/// A resource file header.
pub(crate) struct FileHeader {
    /// A signature identifying this file as a valid resource file.
    signature: [u8; 16],
    /// A user comment.
    comment: [u8; 96],
    /// Reserved for future use.
    reserved: [u8; 12],
    /// A file offset to the directory header.
    dir_header_offset: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
/// A resource file directory header.
pub(crate) struct DirectoryHeader {
    /// The number of items in the directory list.
    num_entries: u16,
    /// A file offset to where the data segment resides.
    data_offset: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
/// A resource file directory entry.
pub(crate) struct DirectoryEntry {
    /// The resource ID.
    id: u16,
    /// The uncompressed length of the resource in bytes.
    uncompressed_len: U24,
    /// The resource flags.
    flags: ResourceFlags,
    /// The compressed length of the resource in bytes.
    compressed_len: U24,
    /// The resource type.
    res_type: ResourceType,
}

#[derive(Clone, Copy, Debug)]
/// A 24-bit unsigned integer.
struct U24([u8; 3]);

/// Interprets a byte array as a 16-bit unsigned little endian integer.
fn u16_from_le_array(array: [u8; 2]) -> u16 {
    let b1 = u16::from(array[1]) << 8;
    let b0 = u16::from(array[0]);
    b1 | b0
}

/// Interprets a byte array as a 32-bit unsigned little endian integer.
fn u32_from_le_array(array: [u8; 4]) -> u32 {
    let b3 = u32::from(array[3]) << 24;
    let b2 = u32::from(array[2]) << 16;
    let b1 = u32::from(array[1]) << 8;
    let b0 = u32::from(array[0]);
    b3 | b2 | b1 | b0
}

impl ResourceType {
    /// Casts a `u8` into its equivalent resource type, or `Unknown` if there is no equivalent.
    pub(crate) fn from_u8(other: u8) -> Self {
        match other {
            1 => ResourceType::String,
            2 => ResourceType::Image,
            3 => ResourceType::Font,
            4 => ResourceType::AnimationScript,
            5 => ResourceType::Palette,
            6 => ResourceType::ShadingTable,
            7 => ResourceType::Voc,
            8 => ResourceType::Shape,
            9 => ResourceType::Picture,
            10 => ResourceType::Babl2Extern,
            11 => ResourceType::Babl2Reloc,
            12 => ResourceType::Babl2Code,
            13 => ResourceType::Babl2Header,
            14 => ResourceType::Babl2Reserved,
            15 => ResourceType::Object3d,
            16 => ResourceType::Stencil,
            17 => ResourceType::Movie,
            18 => ResourceType::Rectangle,
            48 => ResourceType::AppDefined0,
            49 => ResourceType::AppDefined1,
            50 => ResourceType::AppDefined2,
            51 => ResourceType::AppDefined3,
            52 => ResourceType::AppDefined4,
            53 => ResourceType::AppDefined5,
            54 => ResourceType::AppDefined6,
            55 => ResourceType::AppDefined7,
            56 => ResourceType::AppDefined8,
            57 => ResourceType::AppDefined9,
            58 => ResourceType::AppDefined10,
            59 => ResourceType::AppDefined11,
            60 => ResourceType::AppDefined12,
            61 => ResourceType::AppDefined13,
            62 => ResourceType::AppDefined14,
            63 => ResourceType::AppDefined15,
            _ => ResourceType::Unknown,
        }
    }
}

impl FileHeader {
    #[inline]
    /// Reads a file header from an arbitrary input stream.
    ///
    /// The input stream is assumed to be positioned at the first byte of a file header.
    pub(crate) fn from_reader<R: Read>(mut source: R) -> Result<Self, Error> {
        let mut buffer = [0; mem::size_of::<Self>()];
        let mut header = Self {
            signature: [0; 16],
            comment: [0; 96],
            reserved: [0; 12],
            dir_header_offset: 0,
        };
        source.read_exact(&mut buffer).map_err(Error::IO)?;
        header.signature.copy_from_slice(&buffer[..16]);
        header.comment.copy_from_slice(&buffer[16..112]);
        header.reserved.copy_from_slice(&buffer[112..124]);
        header.dir_header_offset =
            u32_from_le_array([buffer[124], buffer[125], buffer[126], buffer[127]]);
        if &header.signature == b"LG Res File v2\x0D\x0A" {
            Ok(header)
        } else {
            Err(Error::BadSignature)
        }
    }

    /// Returns the user comment stored in a header file as a byte slice.
    pub(crate) fn comment(&self) -> &[u8] {
        &self.comment
    }

    /// Returns the user comment stored in a header file as a string slice.
    pub(crate) fn comment_str(&self) -> Result<&str, Error> {
        str::from_utf8(&self.comment).map_err(Error::Utf8)
    }

    /// Returns the file offset of the directory header.
    pub(crate) fn dir_header_offset(&self) -> u64 {
        self.dir_header_offset.into()
    }
}

impl Debug for FileHeader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let dho = self.dir_header_offset;
        f.debug_struct("FileHeader")
            .field("signature", &self.signature)
            .field("comment", &&self.comment[..])
            .field("reserved", &self.reserved)
            .field("dir_header_offset", &dho)
            .finish()
    }
}

impl DirectoryHeader {
    /// Reads a directory header from an arbitrary input stream.
    ///
    /// The input stream is assumed to be positioned at the first byte of a directory header.
    pub(crate) fn from_reader<R: Read>(mut source: R) -> Result<Self, Error> {
        let mut buffer = [0; mem::size_of::<Self>()];
        source.read_exact(&mut buffer).map_err(Error::IO)?;
        Ok(Self {
            num_entries: u16_from_le_array([buffer[0], buffer[1]]),
            data_offset: u32_from_le_array([buffer[2], buffer[3], buffer[4], buffer[5]]),
        })
    }

    /// Returns the number of entries in the directory list.
    pub(crate) fn num_entries(&self) -> usize {
        self.num_entries.into()
    }

    /// Returns a file offset to the location of the file data segment.
    pub(crate) fn data_offset(&self) -> u64 {
        self.data_offset.into()
    }
}

impl DirectoryEntry {
    /// Reads a directory entry from an arbitrary input stream.
    ///
    /// The input stream is assumed to be positioned at the first byte of a directory entry.
    pub(crate) fn from_reader<R: Read>(mut source: R) -> Result<Self, Error> {
        let mut buffer = [0; mem::size_of::<Self>()];
        source.read_exact(&mut buffer).map_err(Error::IO)?;
        Ok(Self {
            id: u16_from_le_array([buffer[0], buffer[1]]),
            uncompressed_len: U24::from_le_array([buffer[2], buffer[3], buffer[4]]),
            flags: ResourceFlags::from_bits_truncate(buffer[5]),
            compressed_len: U24::from_le_array([buffer[6], buffer[7], buffer[8]]),
            res_type: ResourceType::from_u8(buffer[9]),
        })
    }

    /// Returns the ID of a directory entry.
    pub(crate) fn id(&self) -> usize {
        self.id.into()
    }

    /// Returns `true` if this is a "deleted" directory entry, or `false` otherwise.
    pub(crate) fn is_deleted(&self) -> bool {
        self.id() == 0
    }

    /// Returns the length in bytes of a directory entry's corresponding resource when loaded into
    /// memory.
    pub(crate) fn uncompressed_len(&self) -> usize {
        self.uncompressed_len.value() as usize
    }

    /// Returns the flags associated with a directory entry's corresponding resource.
    pub(crate) fn flags(&self) -> ResourceFlags {
        self.flags
    }

    /// Returns the length in bytes of a directory entry's corresponding resource when stored on
    /// physical media.
    pub(crate) fn compressed_len(&self) -> usize {
        self.compressed_len.value() as usize
    }

    /// Returns the type of a directory entry's corresponding resource.
    pub(crate) fn resource_type(&self) -> ResourceType {
        self.res_type
    }
}

impl U24 {
    /// Interprets a byte array as a 24-bit unsigned little-endian integer.
    pub(crate) fn from_le_array(array: [u8; 3]) -> Self {
        U24(array)
    }

    /// Casts a 24-bit unsigned integer into a 32-bit unsigned integer.
    pub(crate) fn value(&self) -> u32 {
        let b2 = u32::from(self.0[2]) << 16;
        let b1 = u32::from(self.0[1]) << 8;
        let b0 = u32::from(self.0[0]);
        b2 | b1 | b0
    }
}
