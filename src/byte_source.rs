use std::fs::File;
use std::io::Read;

use error::Result;

use std::iter::Iterator;

pub type ByteSource = Iterator<Item = Result<u8>>;

pub struct FileByteSource {
    file: File
}

impl Iterator for FileByteSource {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        match self.file.bytes().next() {
            Some(data) => match data {
                Ok(data) => Some(Ok(data)),
                Err(e) => Some(Err(e.into()))
            },
            None => None
        }
    }
}


pub struct VecByteSource {
    data: Iterator<Item = u8>
}

impl Iterator for VecByteSource {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        match self.data.next() {
            Some(data) => Some(Ok(data)),
            None => None
        }
    }
}
