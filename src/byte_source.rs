use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use error::{Result, ErrorKind, ResultExt};

#[derive(Clone)]
pub enum ByteSource {
    File(PathBuf),
    Memory(Vec<u8>)
}

pub fn vec_from_byte_source(source: ByteSource) -> Result<Vec<u8>> {
    match source {
        ByteSource::File(path) => {
            let mut file = File::open(&path)
                .chain_err(||ErrorKind::ByteSourceExpansionFailed)?;
            let mut buffer = vec!();
            file.read_to_end(&mut buffer)
                .chain_err(||ErrorKind::ByteSourceExpansionFailed)?;

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
        let bytesource = ByteSource::Memory(vec!(0,1,2,3));

        assert_eq!(vec_from_byte_source(bytesource).unwrap(), vec!(0,1,2,3));
    }

    #[test]
    fn file_byte_source() {
        let bs = ByteSource::File(PathBuf::from("test/files/exif1.txt"));

        assert_eq!(
            vec_from_byte_source(bs).unwrap(),
            include_bytes!("../test/files/exif1.txt")
                .into_iter()
                .map(|x| *x)
                .collect::<Vec<_>>()
        );
    }
}
