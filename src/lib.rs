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

pub use metadata::{ResourceFlags, ResourceType};

use metadata::{DirectoryEntry, DirectoryHeader, FileHeader};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Read, Seek, SeekFrom};
use std::str;

/// A resource file reader.
pub struct Reader<R> {
    /// The input stream from which to read.
    source: R,
    /// A copy of the file header.
    file_header: FileHeader,
    /// A file offset to the beginning of the data segment.
    data_offset: u64,
    /// A copy of the directory list.
    directory_list: Box<[DirectoryEntry]>,
    /// A list of previously-loaded resources.
    resources: HashMap<usize, Resource>,
}

#[derive(Clone, Debug)]
/// A resource.
pub struct Resource {
    /// The resource type.
    res_type: ResourceType,
    /// Resource flags.
    flags: ResourceFlags,
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
        let _ = source.seek(SeekFrom::Start(0)).map_err(Error::IO)?;
        let file_header = FileHeader::from_reader(&mut source)?;
        let _ = source
            .seek(SeekFrom::Start(file_header.dir_header_offset()))
            .map_err(Error::IO)?;
        let dir_header = DirectoryHeader::from_reader(&mut source)?;
        let mut dir_list = Vec::with_capacity(dir_header.num_entries());
        let mut preload_list = Vec::with_capacity(dir_header.num_entries());
        for _ in 0..dir_header.num_entries() {
            let entry = DirectoryEntry::from_reader(&mut source)?;
            if !entry.is_deleted() && entry.flags().contains(ResourceFlags::RDF_LOADONOPEN) {
                preload_list.push(entry);
            }
            dir_list.push(entry);
        }
        let mut reader = Self {
            source,
            file_header,
            data_offset: dir_header.data_offset(),
            directory_list: dir_list.into_boxed_slice(),
            resources: HashMap::new(),
        };
        for entry in preload_list {
            reader.load_resource(entry.id())?;
        }
        Ok(reader)
    }

    /// Returns a byte slice containing the user comment associated with a resource file.
    pub fn comment(&self) -> &[u8] {
        self.file_header.comment()
    }

    /// Returns a string containing the user comment associated with a resource file.
    pub fn comment_str(&self) -> Result<&str, Error> {
        self.file_header.comment_str()
    }

    /// Returns `true` if a resource file contains a resource with the specified ID, or `false`
    /// otherwise.
    ///
    /// # Panics
    ///
    /// This method panics if the given ID is `0`, because that value is reserved for "deleted"
    /// entries within a resource file's directory list. Use `Reader::contains_deleted_entries` to
    /// determine if a resource file's directory list contains deleted entries.
    pub fn contains_id(&self, id: usize) -> bool {
        assert_ne!(id, 0);
        self.directory_list.iter().any(|entry| entry.id() == id)
    }

    /// Returns `true` if a resource file's directory list contains "deleted" entries, or `false`
    /// otherwise.
    pub fn contains_deleted_entries(&self) -> bool {
        self.directory_list.iter().any(|entry| entry.id() == 0)
    }

    /// Returns a reference to a given resource.
    ///
    /// This will load the resource into memory, if necessary.
    ///
    /// # Panics
    ///
    /// This method panics if the given ID is `0`, because that value is reserved for "deleted"
    /// entries within a resource file's directory list. Use `Reader::contains_deleted_entries` to
    /// determine if a resource file's directory list contains deleted entries.
    pub fn resource(&mut self, id: usize) -> Result<&Resource, Error> {
        self.load_resource(id).map(move |_| {
            if let Some(res) = self.loaded_resource(id) {
                res
            } else {
                unreachable!()
            }
        })
    }

    /// Returns a reference to a given resource if it is currently loaded into memory, or `None`
    /// otherwise.
    ///
    /// This does not check if the given resource actually exists within the resource file.
    ///
    /// # Panics
    ///
    /// This method panics if the given ID is `0`, because that value is reserved for "deleted"
    /// entries within a resource file's directory list. Use `Reader::contains_deleted_entries` to
    /// determine if a resource file's directory list contains deleted entries.
    pub fn loaded_resource(&self, id: usize) -> Option<&Resource> {
        assert_ne!(id, 0);
        self.resources.get(&id)
    }

    /// Loads a resource into memory.
    ///
    /// If the resource is already loaded, this method returns successfully without taking further
    /// action.
    ///
    /// # Panics
    ///
    /// This method panics if the given ID is `0`, because that value is reserved for "deleted"
    /// entries within a resource file's directory list. Use `Reader::contains_deleted_entries` to
    /// determine if a resource file's directory list contains deleted entries.
    pub fn load_resource(&mut self, id: usize) -> Result<(), Error> {
        assert_ne!(id, 0);
        if self.resources.contains_key(&id) {
            return Ok(());
        }
        let mut file_offset = self.data_offset;
        for entry in self.directory_list.iter() {
            if entry.id() == id {
                if entry.flags().contains(ResourceFlags::RDF_LZW) {
                    unimplemented!()
                }
                let _ = self.source
                    .seek(SeekFrom::Start(file_offset))
                    .map_err(Error::IO)?;
                let mut buffer = vec![0; entry.uncompressed_len()].into_boxed_slice();
                self.source.read_exact(&mut buffer).map_err(Error::IO)?;
                let _ = self.resources.insert(
                    id,
                    Resource {
                        res_type: entry.resource_type(),
                        flags: entry.flags(),
                        data: buffer,
                    },
                );
                return Ok(());
            }
            file_offset += entry.compressed_len() as u64;
        }
        Err(Error::ResourceNotFound)
    }
}

impl<R: Debug> Debug for Reader<R> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Reader")
            .field("source", &self.source)
            .field("file_header", &self.file_header)
            .field("data_offset", &self.data_offset)
            .field("directory_list", &self.directory_list)
            .field("resources", &self.resources)
            .finish()
    }
}
