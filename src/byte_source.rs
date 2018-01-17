use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use error::Result;

#[derive(Clone)]
pub enum ByteSource {
    File(PathBuf),
    Memory(Vec<u8>)
}

pub fn vec_from_byte_source(source: ByteSource) -> Result<Vec<u8>> {
    match source {
        ByteSource::File(path) => {
            let mut file = File::open(&path)?;
            let mut buffer = vec!();
            file.read_to_end(&mut buffer)?;

            Ok(buffer)
        },
        ByteSource::Memory(vec) => Ok(vec)
    }
}

pub fn write_byte_source_to_file(source: ByteSource, path: &Path) -> Result<()> {
    let content = vec_from_byte_source(source)?;

    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_byte_source() {
        let mut bytesource = ByteSource::Memory(vec!(0,1,2,3));

        assert_eq!(vec_from_byte_source(&mut bytesource).unwrap(), vec!(0,1,2,3));
    }

    #[test]
    fn file_byte_source() {
        let mut bs = ByteSource::File(PathBuf::from("../test/files/exif1.txt"));

        assert_eq!(
            drain_byte_source(&mut bs).unwrap(),
            include_bytes!("../test/files/exif1.txt")
                .into_iter()
                .map(|x| *x)
                .collect::<Vec<_>>()
        );
    }
}
