extern crate image;
extern crate rand;
extern crate immeta;

use std::path::{Path, PathBuf};

use std::fs::File;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use std::thread;

use glob::glob;


/**
  Enum for different types of media
*/
#[derive(Debug, Eq, PartialEq)]
pub enum MediaType {
    Image,
    Video,
}

/**
  Returns the filetype of a specified file based on its extension
*/
pub fn get_mediatype(path: &Path) -> MediaType {
    let extension = path.extension().unwrap();

    match extension.to_string_lossy().to_lowercase().as_str() {
        "jpg" | "png" | "gif" => MediaType::Image,
        "mov" | "mp4" | "webm" => MediaType::Video,
        _ => {
            println!(
                "Unrecognised extension: {} assuming image",
                extension.to_string_lossy()
            );
            MediaType::Image
        }
    }
}

/**
  Generates a thumbnail for the given source file and stores that file in a unique location which
  is returned by the function.

  If the thumbnail generation fails for any reason it will return an error

  The `max_size` variable is the biggest allowed size on either axis.
  An image in portrait mode will be at most `max_size` tall and an image in
  landscape mode will be at most `max_width` tall
 */

//TODO: Send errors back to caller over a channel instead of ignoring them
pub fn generate_thumbnail(
    source_path: &Path,
    destination_path: &Path,
    max_size: u32,
) -> Result<(), image::ImageError> {
    {
        let destination_path = destination_path.to_owned();
        let source_path_clone = source_path.to_owned();
        thread::spawn(move || {
            let img = match image::open(source_path_clone) {
                Ok(val) => val,
                Err(_) => return,
            };

            let thumb_data = generate_thumbnail_from_generic_image(&img, max_size);

            let fout = &mut File::create(&destination_path).unwrap();
            thumb_data.save(fout, image::PNG).unwrap();
        });
    }

    Ok(())
}


pub fn get_file_extension(path: &Path) -> String {
    match path.extension() {
        Some(val) => ".".to_string() + val.to_str().unwrap(),
        None => "".to_string(),
    }
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


pub fn system_time_as_unix_timestamp(time: SystemTime) -> u64 {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();

    duration.as_secs()
}

/**
    Returns the unix timestamp of an image.

    For now, this is the timestamp of the file
    in the file system because there is no good library for reading EXIF data.

    If the file doesn't exist, an error is printed and `SystemTime::now()` is returned
*/
//TODO: Rewrite to return an option
pub fn get_file_timestamp(filename: &PathBuf) -> u64 {
    let metadata = match fs::metadata(&filename) {
        Ok(val) => val,
        Err(e) => {
            println!(
                "Failed to load image timestamp for file {:?}. {:?}",
                filename,
                e
            );
            return system_time_as_unix_timestamp(SystemTime::now());
        }
    };

    let timestamp = metadata.modified().unwrap();

    system_time_as_unix_timestamp(timestamp)
}


/**
  Checks a list of tags for unallowed characters and converts it into a storeable format,
  which at the moment is just removal of capital letters
 */
pub fn sanitize_tag_names(tag_list: &[String]) -> Result<Vec<String>, String> {
    let mut new_list = vec![];

    for tag in tag_list {
        if tag == "" {
            return Err(String::from("Tags can not be empty"));
        }

        new_list.push(tag.to_lowercase());
    }

    Ok(new_list)
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

    #[test]
    fn sanitize_tests() {
        {
            let vec = vec![
                String::from("abCde"),
                String::from("ABC"),
                String::from("abc"),
            ];

            let expected = vec![
                String::from("abcde"),
                String::from("abc"),
                String::from("abc"),
            ];

            assert_eq!(sanitize_tag_names(&vec), Ok(expected));
        }

        {
            assert_eq!(
                sanitize_tag_names(&vec![String::from("")]),
                Err(String::from("Tags can not be empty"))
            );
        }
    }

    #[test]
    fn file_type_test() {
        assert_eq!(get_mediatype(&PathBuf::from("yolo.jpg")), MediaType::Image);
        assert_eq!(get_mediatype(&PathBuf::from("yolo.png")), MediaType::Image);
        assert_eq!(get_mediatype(&PathBuf::from("yolo.mov")), MediaType::Video);
        assert_eq!(get_mediatype(&PathBuf::from("yolo.mp4")), MediaType::Video);

        assert_eq!(get_mediatype(&PathBuf::from("yolo.MOV")), MediaType::Video);
        assert_eq!(
            get_mediatype(&PathBuf::from("some/path.yoloswag/1234/yolo.MOV")),
            MediaType::Video
        );
    }
}
