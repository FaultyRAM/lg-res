// Copyright (c) 2018 FaultyRAM
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the
// MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at
// your option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for Looking Glass RES archives.

#![forbid(warnings)]
#![forbid(future_incompatible)]
#![deny(unused)]
#![forbid(missing_copy_implementations)]
#![forbid(missing_debug_implementations)]
#![forbid(missing_docs)]
#![forbid(trivial_casts)]
#![forbid(trivial_numeric_casts)]
#![forbid(unreachable_pub)]
#![forbid(unsafe_code)]
#![forbid(unused_import_braces)]
#![deny(unused_qualifications)]
#![forbid(unused_results)]
#![forbid(variant_size_differences)]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", forbid(clippy))]
#![cfg_attr(feature = "clippy", forbid(clippy_complexity))]
#![cfg_attr(feature = "clippy", forbid(clippy_correctness))]
#![cfg_attr(feature = "clippy", forbid(clippy_pedantic))]
#![cfg_attr(feature = "clippy", forbid(clippy_perf))]
#![cfg_attr(feature = "clippy", forbid(clippy_style))]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;

mod metadata;

pub use metadata::{DirectoryEntry, ResourceFlags, ResourceType};

use metadata::{DirectoryHeader, DirectoryList, FileHeader};
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read, Seek, SeekFrom};
use std::str;

/// A resource file reader.
pub struct Reader<R> {
    /// The input stream from which to read.
    source: R,
    /// A copy of the file header.
    file_header: FileHeader,
    /// A copy of the directory list, where each entry is mapped to the file offset of its
    /// corresponding resource.
    directory_list: DirectoryList,
}

#[derive(Clone, Debug)]
/// A resource.
pub struct Resource {
    /// A copy of the directory entry for this resource.
    metadata: DirectoryEntry,
    /// The resource itself.
    data: Box<[u8]>,
}

#[derive(Debug, Fail)]
/// A type that represents errors arising from this crate.
pub enum Error {
    #[fail(display = "I/O error: {}", _0)]
    /// An I/O error.
    IO(io::Error),
    #[fail(display = "invalid UTF-8 sequence: {}", _0)]
    /// A UTF-8 interpretation error.
    Utf8(str::Utf8Error),
    #[fail(display = "invalid resource file signature")]
    /// A resource file began with an invalid or unrecognised signature.
    BadSignature,
    #[fail(display = "resource not found")]
    /// A specified resource could not be retrieved.
    ResourceNotFound,
}

impl<R: Read + Seek> Reader<R> {
    /// Creates a new resource file reader over an input stream.
    ///
    /// The input stream's position is moved to the beginning of the stream before any reads are
    /// performed.
    pub fn new(mut source: R) -> Result<Self, Error> {
        let file_header = source
            .seek(SeekFrom::Start(0))
            .map_err(Error::IO)
            .and_then(|_| FileHeader::from_reader(&mut source))?;
        source
            .seek(SeekFrom::Start(file_header.dir_header_offset()))
            .map_err(Error::IO)
            .and_then(|_| DirectoryHeader::from_reader(&mut source))
            .and_then(|dir_header| DirectoryList::from_reader(dir_header, &mut source))
            .map(|directory_list| Self {
                source,
                file_header,
                directory_list,
            })
    }

    /// Returns a byte slice containing the user comment associated with a resource file.
    pub fn comment(&self) -> &[u8] {
        self.file_header.comment()
    }

    /// Returns a string containing the user comment associated with a resource file.
    pub fn comment_str(&self) -> Result<&str, Error> {
        self.file_header.comment_str()
    }

    /// Returns the directory list for a resource file.
    ///
    /// The directory list is a series of entries where each entry contains some metadata for its
    /// corresponding resource. Rather than storing file offsets, a resource file stores resource
    /// lengths and assumes that resources are laid out in the same order as their corresponding
    /// entries in the directory list.
    pub fn directory_list(&self) -> &[DirectoryEntry] {
        self.directory_list.entries()
    }

    /// Returns the first resource from a resource file whose corresponding entry in the directory
    /// list matches a given predicate.
    pub fn find_resource<P: Fn(&DirectoryEntry) -> bool>(
        &mut self,
        predicate: P,
    ) -> Result<Resource, Error> {
        let (index, entry) = self.directory_list
            .entries()
            .iter()
            .enumerate()
            .find(|(_, entry)| (predicate)(&entry))
            .ok_or(Error::ResourceNotFound)?;
        if entry.is_compound_resource() || entry.is_compressed() {
            unimplemented!()
        }
        let mut buffer = self.source
            .seek(SeekFrom::Start(self.directory_list.offset_for_index(index)))
            .map(|_| vec![0; entry.decompressed_len()].into_boxed_slice())
            .map_err(Error::IO)?;
        self.source
            .read_exact(&mut buffer)
            .map(|_| Resource {
                metadata: *entry,
                data: buffer,
            })
            .map_err(Error::IO)
    }
}

impl<R: Debug> Debug for Reader<R> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Reader")
            .field("source", &self.source)
            .field("file_header", &self.file_header)
            .field("directory_list", &self.directory_list)
            .finish()
    }
}
