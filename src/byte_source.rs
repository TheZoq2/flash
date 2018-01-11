use std::fs::File;
use std::io::Read;

use error::Result;

use std::iter::Iterator;

pub type ByteSource = Iterator<Item = Result<u8>> + Sync + Send;

pub struct FileByteSource {
    pub file: File
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
    pub data: Vec<u8>
}

impl Iterator for VecByteSource {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        match self.data.pop() {
            Some(data) => Some(Ok(data)),
            None => None
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn drain_byte_source<B: ByteSource>(bs: &mut B) -> Result<Vec<u8>> {
        let mut result = vec!();
        while let Some(val) = bs.next() {
            result.push(val)?;
        }
        Ok(result)
    }

    #[test]
    fn vec_byte_source() {
        let mut bytesource = VecByteSource{data: vec!(0,1,2,3)};

        assert_eq!(drain_byte_source(&mut bytesource), vec!(0,1,2,3));
    }

    #[test]
    fn file_byte_source() {
        let mut file = File::open("test/files/exif1.txt").expect("test/files/exif1.txt does not exist");

        let bs = FileByteSource{file};

        assert_eq!(drain_byte_source(bs), include_bytes!("test/files/exif1.txt"));
    }
}
