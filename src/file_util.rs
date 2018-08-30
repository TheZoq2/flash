extern crate image;
extern crate rand;
extern crate immeta;

use std::path::{Path, PathBuf};

use std::fs::File;
use std::fs;

use std::thread;

use glob::glob;

use error::{Result, ResultExt, ErrorKind};

use chrono::NaiveDateTime;

use exiftool;
use exiftool::{ExifData};
use byte_source::{ByteSource, vec_from_byte_source};

use std::sync::mpsc;

use std::time::{SystemTime, UNIX_EPOCH};

/**
  Length or width, depending on which is the longest of a generated thumbnail
*/
const THUMBNAIL_SIZE: u32 = 200;

/**
  Starts a thread that generates a thumbnail from the specified source. It is
  stored in `destination_path`

  Returns a channel which can be used to listen for errors that occured during
  generation
 */

pub fn generate_thumbnail(
    source: ByteSource,
    destination_path: &Path,
) -> mpsc::Receiver<Result<()>> {
    let destination_path = destination_path.to_owned();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let handler = || -> Result<()> {
            let file_content = vec_from_byte_source(source)?;
            let img = image::load_from_memory(&file_content)?;

            let thumb_data = generate_thumbnail_from_generic_image(&img, THUMBNAIL_SIZE);

            let fout = &mut File::create(&destination_path)?;
            thumb_data.save(fout, image::PNG)?;
            Ok(())
        };

        let generation_result = handler()
            .chain_err(|| ErrorKind::ThumbnailGenerationFailed);

        // We don't care if the result could not be sent because it probably means
        // that the receiver does not care
        let _result = tx.send(generation_result);
    });

    rx
}


/**
  Takes a `image::GenericImage` and generates a thumbnail image from that
 */
fn generate_thumbnail_from_generic_image(
    src: &image::DynamicImage,
    max_size: u32,
) -> image::DynamicImage {
    src.resize(max_size, max_size, image::FilterType::Nearest)
}

/**
  Returns a really big random number as a string
 */
pub fn get_semi_unique_identifier() -> i32 {
    rand::random::<i32>()
}



pub fn get_file_timestamp(filename: &Path) -> Result<NaiveDateTime> {
    Ok(match get_file_timestamp_from_metadata(filename)? {
        Some(timestamp) => timestamp,
        None => get_file_timestamp_from_filesystem(filename)?
    })
}

/**
  Reads the timestamp of the specified file from the file metadata. Returns Ok(None) if
  the metadata does not contain a creation time and Err if reading failed for some reason.
*/
pub fn get_file_timestamp_from_metadata(filename: &Path) -> Result<Option<NaiveDateTime>> {
    let exif_data = ExifData::from_file(&filename.to_string_lossy())?;

    match exif_data.get_creation_date() {
        Ok(data) => Ok(Some(data)),
        Err(exiftool::Error(exiftool::ErrorKind::NoSuchTag(_), _)) => Ok(None),
        Err(e) => Err(e)?
    }
}


/**
  Converts a `SystemTime` into a u64
*/
fn system_time_as_unix_timestamp(time: SystemTime) -> Result<u64> {
    let duration = time.duration_since(UNIX_EPOCH)?;

    Ok(duration.as_secs())
}

/**
  Reads the modified time of the specified file from the timestamp in the file system.
*/
fn get_file_timestamp_from_filesystem(filename: &Path) -> Result<NaiveDateTime> {
    let modification_time = fs::metadata(filename)?.modified()?;

    Ok(NaiveDateTime::from_timestamp(system_time_as_unix_timestamp(modification_time)? as i64, 0))
}


/**
  Checks a list of tags for unallowed characters and converts it into a storeable format,
  which at the moment is just removal of capital letters
 */
pub fn sanitize_tag_names(tag_list: &[String]) -> Vec<String> {
    tag_list.iter().filter(|x| !x.is_empty()).map(|x| x.to_lowercase()).collect()
}


/**
  Returns a list of all the files in a directory
*/
pub fn get_files_in_dir(dir: &PathBuf) -> Vec<PathBuf> {
    let mut result = Vec::<PathBuf>::new();

    let full_path = String::from(dir.to_string_lossy()).clone() + "/*";

    for entry in glob(&full_path).expect("Failed to read glob") {
        match entry {
            Ok(path) => result.push(path),
            Err(e) => println!("{}", e),
        }
    }

    result
}

/**
  Returns a list of all subdirectories of a dir
*/
pub fn subdirs_in_directory(dir: &Path) -> Result<Vec<PathBuf>> {
    let files = fs::read_dir(dir)?;

    Ok(files.map(|file| {
            let file = file?;
            match file.metadata() {
                Ok(ref metadata) if metadata.is_dir() => Ok(Some(file.path())),
                Ok(_) => Ok(None),
                Err(e) => Err(e)?
            }
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|a| a)
        .map(|path| path.strip_prefix(dir).unwrap().to_path_buf())
        .collect()
    )
}


#[cfg(test)]
mod thumbnail_tests {
    extern crate image;
    use image::GenericImage;

    #[test]
    fn thumbnail_test() {
        let img = image::DynamicImage::new_rgba8(500, 500);

        let thumbnail = super::generate_thumbnail_from_generic_image(&img, 300);

        assert!(thumbnail.dimensions() == (300, 300));
    }
    #[test]
    fn portrait_test() {
        let img = image::DynamicImage::new_rgba8(500, 250);

        let thumbnail = super::generate_thumbnail_from_generic_image(&img, 300);

        assert!(thumbnail.dimensions() == (300, 150));
    }

    #[test]
    fn landscape_test() {
        let img = image::DynamicImage::new_rgba8(250, 500);

        let thumbnail = super::generate_thumbnail_from_generic_image(&img, 300);

        assert!(thumbnail.dimensions() == (150, 300));
    }

}


#[cfg(never)]
mod thumbnail_bench {
    extern crate test;
    use self::test::Bencher;

    extern crate image;


    #[bench]
    fn thumbnail_generation_bench(b: &mut Bencher) {
        let image = image::open("test/media/DSC_0001.JPG").unwrap();

        b.iter(|| {
            super::generate_thumbnail_from_generic_image(&image, 300);
        })
    }
}


#[cfg(test)]
mod util_tests {
    use super::*;

    use chrono::NaiveDate;

    #[test]
    fn sanitize_tests() {
        {
            let vec = vec![
                String::from("abCde"),
                String::from("ABC"),
                String::from("abc"),
                String::from(""),
            ];

            let expected = vec![
                String::from("abcde"),
                String::from("abc"),
                String::from("abc"),
            ];

            assert_eq!(sanitize_tag_names(&vec), expected);
        }
    }

    #[test]
    fn subdir_test() {
        let mut subdirs = subdirs_in_directory(&PathBuf::from("test")).expect("Failed to read files in dir");

        let mut expected = mapvec!(PathBuf::from: "media", "files");

        expected.sort();
        subdirs.sort();

        assert_eq!(subdirs.len(), 2);
        assert_eq!(
            subdirs,
            expected
        );
    }

    #[test]
    fn metadata_timestamp_test() {
        let path = PathBuf::from("test/media/DSC_0001.JPG");

        let timestamp = get_file_timestamp(&path).unwrap();

        assert_eq!(timestamp, NaiveDate::from_ymd(2016, 12, 16).and_hms(21, 34, 26));
    }

    #[test]
    // TODO Test disabled for now. Find a way to reliably test this
    fn filesystem_timestamp_test() {
        let path = PathBuf::from("test/media/10x10.png");

        let timestamp = get_file_timestamp(&path).unwrap();

        // assert_eq!(timestamp, NaiveDate::from_ymd(2018, 5, 31).and_hms(20, 39, 56));
    }
}
