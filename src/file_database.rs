extern crate image;

use std::vec::Vec;
use std::collections::HashMap;

use rustc_serialize::json;

use std::io::prelude::*;
use std::io;
use std::fs::{File, OpenOptions};
use std::thread;
use std::fs;

use std::path::Path;

use settings::Settings;

use iron::typemap::Key;

use image::{GenericImage};

/**
  A reference to a file stored in the file database
 */
#[derive(RustcEncodable, RustcDecodable, Clone)]
pub struct FileEntry
{
    //The unique ID of this file in the db
    id: usize,
    //The path to the actual file
    pub path: String,
    pub tags: Vec<String>,
}
impl FileEntry
{
    pub fn new(id: usize, path: String, tags: Vec<String>) -> FileEntry
    {
        FileEntry {
            id: id,
            path: path,
            tags: tags,
        }
    }
}

//TODO: Move to list of free ids for id reusage
#[derive(RustcEncodable, RustcDecodable)]
pub struct FileDatabase
{
    version: u32,

    next_id: usize,

    //Map from file IDs to actual files
    files: HashMap<usize, FileEntry>,

    //Map from tags to file ids
    tags: HashMap<String, Vec<usize>>,
}

impl FileDatabase
{
    pub fn new() -> FileDatabase
    {
        FileDatabase
        {
            version:0,

            next_id: 0,

            files: HashMap::new(),
            tags: HashMap::new(),
        }
    }
    pub fn load_from_json(storage_path: String) -> FileDatabase
    {
        let mut file = match File::open(&storage_path)
        {
            Ok(file) => file,
            Err(_) => {
                println!("No existing database file found. Creating one in {}", &storage_path);
                return FileDatabase::new();
            }
        };

        let mut json_str = String::new();
        match file.read_to_string(&mut json_str)
        {
            Ok(_) => {},
            Err(e) => {
                println!("Database loading failed. File {} could not be loaded. {}", &storage_path, e);
                println!("Creating a new db file in {}", &storage_path);
                return FileDatabase::new();
            }
        };
        json::decode::<FileDatabase>(&json_str).unwrap()
    }

    /**
      Adds a new file entry to the "database". It is given a new unique ID and the
      file is added to the tags which it should be part of. If some of those tags don't 
      exist yet, then they are added
     */
    pub fn add_new_file(&mut self, path: String, tags: Vec<String>)
    {
        let new_id = self.next_id;

        //Add all the tags to the map
        for tag in &tags
        {
            if self.tags.get(tag) == None
            {
                self.tags.insert(tag.clone(), vec!());
            }

            //self.tags.get_mut(tag).unwrap().push(new_id);
            //Insert the new image using the binary search function.
            let vec = self.tags.get_mut(tag).unwrap();
            match vec.binary_search(&new_id){
                Ok(_) => {
                    //This shouldn't happen
                    println!("ID {} is already part of tag {}. This is an error in FileDatabase::add_new_file", new_id, tag);
                },
                //If binary search returns Err, it means it didn't find an element and it returns
                //the index where the element would be
                Err(new_index) => {vec.insert(new_index, new_id)}
            }
        }

        let file_entry = FileEntry::new(new_id, path, tags);
        self.files.insert(self.next_id, file_entry);

        self.next_id += 1;
    }

    /**
      Returns all FileEntry objects which are part of a specific tag
     */
    pub fn get_files_with_tag(&self, tag: String) -> Vec<FileEntry>
    {
        let ids = match self.tags.get(&tag)
        {
            Some(val) => val.clone(),
            None => Vec::<usize>::new(),
        };

        let mut files = Vec::<FileEntry>::new();
        for id in ids
        {
            //This assumes that the id exists, i'll leave it up to
            //the rest of the system to take care of that.
            files.push(self.files.get(&id).unwrap().clone());
        }

        files
    }
    
    /**
      Returns all files that have all the tags in the list
     */
    pub fn get_files_with_tags(&self, tags: Vec<String>) -> Vec<FileEntry>
    {
        let mut tags = tags.clone();

        if tags.len() == 0
        {
            return vec!();
        }

        let possible_files = self.get_files_with_tag(tags.pop().unwrap());

        let mut result = vec!();

        for file in possible_files
        {
            let mut has_all_tags = true;
            for tag in &tags
            {
                let mut has_tag = false;
                for file_tag in &file.tags
                {
                    if tag == file_tag
                    {
                        has_tag = true;
                    }
                }

                if !has_tag
                {
                    has_all_tags = false
                }
            }

            if has_all_tags
            {
                result.push(file);
            }
        }

        result
    }
    /**
      Returns paths to all file objects that are part of a tag
     */
    pub fn get_file_paths_with_tag(&self, tag: String) -> Vec<String>
    {
        get_file_paths_from_files(self.get_files_with_tag(tag))
    }
    pub fn get_file_paths_with_tags(&self, tag: Vec<String>) -> Vec<String>
    {
        get_file_paths_from_files(self.get_files_with_tags(tag))
    }

    pub fn get_next_id(&self) -> usize
    {
        self.next_id
    }
}

/**
  Keeps track of the current file database and handles loading and saving of it
 */
pub struct FileDatabaseContainer
{
    db: FileDatabase,

    file_path: String,
    db_path: String,
}

impl Key for FileDatabaseContainer { type Value = FileDatabaseContainer; }

impl FileDatabaseContainer
{
    pub fn new(settings_object: &Settings) -> FileDatabaseContainer
    {
        let db = FileDatabase::load_from_json(settings_object.get_database_save_path());

        FileDatabaseContainer {
            //The path to the directory where all the files should be saved
            file_path: settings_object.get_file_storage_path(),
            //The path to the database file
            db_path: settings_object.get_database_save_path(),

            db: db,
        }
    }

    pub fn add_file_to_db(&mut self, path: String, tags: Vec<String>)
    {
        let id = self.db.get_next_id().clone();

        //Figgure out the file extension
        let filename = {
            id.to_string() + &get_file_extention(&path)
        };
        
        let full_fileame = self.file_path.clone() + "/" + &filename;
        thread::spawn(move || {
            //Create a path object from the file path.
            //println!("Saving file to: {}", full_fileame);

            fs::copy(path, full_fileame)
            //TODO: Generate thumbnails
        });

        //Save the file into the database
        self.db.add_new_file(filename, tags);
    }

    pub fn get_db(&self) -> &FileDatabase
    {
        &self.db
    }

    pub fn save(&self) -> Result<(), io::Error>
    {
        let mut file = match OpenOptions::new().write(true).create(true).open(&self.db_path){
            Ok(file) => file,
            Err(e) => 
            {
                println!("Database save to {} failed. {}", self.db_path, e);
                return Err(e)
            }
        };

        file.write_all(json::encode::<FileDatabase>(&self.db).unwrap().as_bytes())
    }
}

pub fn get_file_paths_from_files(files: Vec<FileEntry>) -> Vec<String>
{
    let mut result = vec!();

    for file in files
    {
        result.push(file.path.clone());
    }

    result
}


pub struct ThumbnailInfo
{
    pub path: String,
    pub dimensions: (u32, u32),
}
/**
  Generates a thumbnail for the given source file and stores that file in a unique location which
  is returned by the function. 

  If the thumbnail generation fails for any reason it will return an error

  The result_max_size variable is the biggest allowed size on either axis.
  An image in portrait mode will be at most max_size tall and an image in 
  landscape mode will be at most max_width tall
 */
pub fn generate_thumbnail(source_path: String, max_size: u32) -> Result<ThumbnailInfo, image::ImageError>
{
    let path_obj = &Path::new(&source_path);
    //For now, we assume everything is an image

    //Load the image file
    let img = match image::open(path_obj)
    {
        Ok(val) => val,
        Err(e) => {
            return Err(e);
        }
    };

    let thumb_data = generate_thumbnail_from_generic_image(img, max_size);

    //Generate a filename for the image
    let file_extention = get_file_extention(&source_path);
    let filename = path_obj.file_stem().unwrap().to_str().unwrap();
    let full_path = String::from("/tmp") + filename + &file_extention;

    Ok(ThumbnailInfo
    {
        path: full_path,
        dimensions: thumb_data.dimensions(),
    })
}


fn get_file_extention(path: &String) -> String
{
    let path_obj = Path::new(&path);

    match path_obj.extension(){
        Some(val) => ".".to_string() + val.to_str().unwrap(),
        None => "".to_string()
    }
}

/**
  Takes an image::GenericImage and generates a thumbnail image from that
 */
fn generate_thumbnail_from_generic_image(src: image::DynamicImage, max_size: u32) -> image::DynamicImage
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

/*
   Tests
 */
#[cfg(test)]
mod db_tests
{
    use file_database::*;
    #[test]
    fn add_test()
    {
        let mut fdb = FileDatabase::new();

        fdb.add_new_file("test1".to_string(), vec!("tag1".to_string(), "tag2".to_string()));
        fdb.add_new_file("test2".to_string(), vec!("tag1".to_string(), "tag3".to_string()));

        //Ensure both files are found when searching for tag1
        assert!(fdb.get_file_paths_with_tag("tag1".to_string()).contains(&"test1".to_string()));
        assert!(fdb.get_file_paths_with_tag("tag1".to_string()).contains(&"test2".to_string()));

        //Ensure only the correct tags are found when searching for the other tags
        assert!(fdb.get_file_paths_with_tag("tag2".to_string()).contains(&"test1".to_string()));
        assert!(fdb.get_file_paths_with_tag("tag2".to_string()).contains(&"test2".to_string()) == false);

        assert!(fdb.get_file_paths_with_tag("tag3".to_string()).contains(&"test2".to_string()));
        assert!(fdb.get_file_paths_with_tag("tag3".to_string()).contains(&"test1".to_string()) == false);

        //Ensure that tags that don't exist don't return anything
        assert!(fdb.get_file_paths_with_tag("unused_tag".to_string()).is_empty());
    }

    #[test]
    fn multi_tag_test()
    {
        let mut fdb = FileDatabase::new();

        fdb.add_new_file("test1".to_string(), vec!("common_tag".to_string(), "only1_tag".to_string()));
        fdb.add_new_file("test2".to_string(), vec!("common_tag".to_string(), "only2_3_tag".to_string()));
        fdb.add_new_file("test3".to_string(), vec!("common_tag".to_string(), "only2_3_tag".to_string()));

        let common_2_3 = fdb.get_files_with_tags(vec!("common_tag".to_string(), "only2_3_tag".to_string()));
        assert!(get_file_paths_from_files(common_2_3.clone()).contains(&"test1".to_string()) == false);
        assert!(get_file_paths_from_files(common_2_3.clone()).contains(&"test2".to_string()));
        assert!(get_file_paths_from_files(common_2_3.clone()).contains(&"test3".to_string()));

        let common_1 = fdb.get_files_with_tags(vec!("common_tag".to_string()));
        assert!(get_file_paths_from_files(common_1.clone()).contains(&"test1".to_string()));
        assert!(get_file_paths_from_files(common_1.clone()).contains(&"test2".to_string()));
        assert!(get_file_paths_from_files(common_1.clone()).contains(&"test3".to_string()));

        let only_1 = fdb.get_files_with_tags(vec!("only1_tag".to_string()));
        assert!(get_file_paths_from_files(only_1.clone()).contains(&"test1".to_string()));
        assert!(get_file_paths_from_files(only_1.clone()).contains(&"test2".to_string()) == false);
        assert!(get_file_paths_from_files(only_1.clone()).contains(&"test3".to_string()) == false);

        let none = fdb.get_files_with_tags(vec!());
        assert!(none.len() == 0);
    }
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
