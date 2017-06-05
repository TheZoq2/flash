extern crate image;
extern crate rand;
extern crate immeta;

use image::{GenericImage};

use std::path::{Path, PathBuf};

use std::fs::File;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use std::thread;

use glob::glob;


/**
  Enum for different types of media
*/
#[derive(RustcDecodable, RustcEncodable, Debug, Eq, PartialEq)]
pub enum MediaType
{
    Image,
    Video
}

/**
  Returns the filetype of a specified file based on its extension
*/
pub fn get_mediatype(path: &str) -> MediaType
{
    let extension = get_file_extension(path);

    match extension.to_lowercase().as_str()
    {
        ".jpg" | ".png" | ".gif" => MediaType::Image,
        ".mov" | ".mp4" | ".webm" => MediaType::Video,
        _ => {
            println!("Unrecognised extension: {} assuming image", extension);
            MediaType::Image
        }
    }
}

pub struct ThumbnailInfo
{
    pub path: String,
}
/**
  Generates a thumbnail for the given source file and stores that file in a unique location which
  is returned by the function. 

  If the thumbnail generation fails for any reason it will return an error

  The `max_size` variable is the biggest allowed size on either axis.
  An image in portrait mode will be at most `max_size` tall and an image in 
  landscape mode will be at most `max_width` tall
 */

//TODO: Rewrite to use std::path instead of &str
pub fn generate_thumbnail(source_path: &str, destination_path_without_extension: &str, max_size: u32)
        -> Result<ThumbnailInfo, image::ImageError>
{
    //Generating the filenames
    let file_extension = get_file_extension(source_path);
    let full_path = String::from(destination_path_without_extension) + &file_extension;

    let full_path_clone = full_path.clone();
    let source_path_clone = String::from(source_path);
    thread::spawn(move || {
        let path_obj = Path::new(&source_path_clone);

        let img = match image::open(path_obj)
        {
            Ok(val) => val,
            Err(_) => return
        };

        let thumb_data = generate_thumbnail_from_generic_image(&img, max_size);

        let fout = &mut File::create(&Path::new(&full_path_clone)).unwrap();
        thumb_data.save(fout, image::PNG).unwrap();
    });

    Ok(ThumbnailInfo
    {
        path:full_path
    })
}


pub fn get_file_extension(path: &str) -> String
{
    let path_obj = Path::new(&path);

    match path_obj.extension(){
        Some(val) => ".".to_string() + val.to_str().unwrap(),
        None => "".to_string()
    }
}

/**
  Takes a `image::GenericImage` and generates a thumbnail image from that
 */
fn generate_thumbnail_from_generic_image(src: &image::DynamicImage, max_size: u32) -> image::DynamicImage
{
    //Calculating the dimensions of the new image
    let src_dimensions = src.dimensions();
    let aspect_ratio = src_dimensions.0 as f32 / src_dimensions.1 as f32;

    //If the image is in landscape mode
    let new_dimensions = if aspect_ratio > 1.
    {
        (max_size, (max_size as f32 / aspect_ratio) as u32)
    }
    else
    {
        ((max_size as f32 * aspect_ratio) as u32, max_size)
    };

    //Resize the image
    //image::imageops::resize(&src, new_dimensions.0, new_dimensions.1, image::FilterType::Triangle)
    src.resize_exact(new_dimensions.0, new_dimensions.1, image::FilterType::Triangle)
}

/**
  Returns a really big random number as a string
 */
pub fn get_semi_unique_identifier() -> String
{
    format!("{}", rand::random::<u64>())
}


pub fn system_time_as_unix_timestamp(time: SystemTime) -> u64
{
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
pub fn get_file_timestamp(filename: &PathBuf) -> u64
{
    let metadata = match fs::metadata(&filename)
    {
        Ok(val) => val,
        Err(e) => {
            println!("Failed to load image timestamp for file {:?}. {:?}", filename, e);
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
pub fn sanitize_tag_names(tag_list: &[String]) -> Result<Vec<String>, String>
{
    let mut new_list = vec!();

    for tag in tag_list
    {
        if tag == ""
        {
            return Err(String::from("Tags can not be empty"));
        }

        new_list.push(tag.to_lowercase());
    }

    Ok(new_list)
}


/**
  Returns a list of all the files in a directory
*/
pub fn get_files_in_dir(dir: &PathBuf) -> Vec<PathBuf> 
{
    let mut result = Vec::<PathBuf>::new();

    let full_path = String::from(dir.to_string_lossy()).clone() + "/*";

    for entry in glob(&full_path).expect("Failed to read glob")
    {
        match entry
        {
            Ok(path) => result.push(path),
            Err(e) => println!("{}", e)
        }
    }

    result
}


#[cfg(test)]
mod thumbnail_tests
{
    extern crate image;
    use image::{GenericImage};

    #[test]
    fn thumbnail_test()
    {
        let img = image::DynamicImage::new_rgba8(500, 500);

        let thumbnail = super::generate_thumbnail_from_generic_image(img, 300);

        assert!(thumbnail.dimensions() == (300, 300));
    }
    #[test]
    fn portrait_test()
    {
        let img = image::DynamicImage::new_rgba8(500, 250);

        let thumbnail = super::generate_thumbnail_from_generic_image(img, 300);

        assert!(thumbnail.dimensions() == (300, 150));
    }
    
    #[test]
    fn landscape_test()
    {
        let img = image::DynamicImage::new_rgba8(250, 500);

        let thumbnail = super::generate_thumbnail_from_generic_image(img, 300);

        assert!(thumbnail.dimensions() == (150, 300));
    }

}

#[cfg(test)]
mod util_tests
{
    use super::*;

    #[test]
    fn sanitize_tests()
    {
        {
            let vec = vec!(
                String::from("abCde"), 
                String::from("ABC"), 
                String::from("abc"));

            let expected = vec!(
                String::from("abcde"),
                String::from("abc"),
                String::from("abc")
                );

            assert_eq!(sanitize_tag_names(&vec), Ok(expected));
        }

        {
            assert_eq!(sanitize_tag_names(&vec!(String::from(""))), Err(String::from("Tags can not be empty")));
        }
    }

    #[test]
    fn file_type_test()
    {
        assert_eq!(get_mediatype("yolo.jpg"), MediaType::Image);
        assert_eq!(get_mediatype("yolo.png"), MediaType::Image);
        assert_eq!(get_mediatype("yolo.mov"), MediaType::Video);
        assert_eq!(get_mediatype("yolo.mp4"), MediaType::Video);

        assert_eq!(get_mediatype("yolo.MOV"), MediaType::Video);
        assert_eq!(get_mediatype("some/path.yoloswag/1234/yolo.MOV"), MediaType::Video);
    }
}

