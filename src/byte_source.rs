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
        let mut buffer = [0];

        match self.file.read(&mut buffer) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buffer[0])),
            Err(e) => Some(Err(e.into()))
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

    fn drain_byte_source(bs: &mut ByteSource) -> Result<Vec<u8>> {
        let mut result = vec!();
        while let Some(val) = bs.next() {
            result.push(val?);
        }
        Ok(result)
    }

    #[test]
    fn vec_byte_source() {
        let mut bytesource = VecByteSource{data: vec!(0,1,2,3)};

        assert_eq!(drain_byte_source(&mut bytesource).unwrap(), vec!(0,1,2,3));
    }

    #[test]
    fn file_byte_source() {
        let mut file = File::open("test/files/exif1.txt").expect("test/files/exif1.txt does not exist");

        let mut bs = FileByteSource{file};

        assert_eq!(
            drain_byte_source(&mut bs).unwrap(),
            include_bytes!("../test/files/exif1.txt")
                .into_iter()
                .map(|x| *x)
                .collect::<Vec<_>>()
        );
    }
}
