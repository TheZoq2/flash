
extern crate image;
extern crate rand;

use std::vec::Vec;
use std::collections::HashMap;

use rustc_serialize::json;

use std::io::prelude::*;
use std::io;
use std::fs::{File, OpenOptions};

use settings::Settings;

use iron::typemap::Key;

//use std::collections::Bound::{Included};


//use std::iter::Iterator;
use std::vec::IntoIter;


pub enum TimestampRange
{
    Unbounded,
    Bounded(u64)
}


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

    pub timestamp: u64,

    pub thumbnail_path: String,

    pub additional_data: HashMap<String, String>,
}
impl FileEntry
{
    pub fn new(id: usize, path: String, tags: Vec<String>, thumbnail_path: String, timestamp: u64) -> FileEntry
    {
        FileEntry {
            id: id,
            path: path,
            tags: tags,

            timestamp: timestamp,

            thumbnail_path: thumbnail_path,

            additional_data: HashMap::new(),
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

      Returns the ID of the added image
     */
    pub fn add_new_file(&mut self, filename: &String, thumb_name: &String, 
                        tags: &Vec<String>, timestamp: u64) -> usize
    {
        let new_id = self.next_id;

        //Add all the tags to the map
        for tag in tags
        {
            if self.tags.get(tag) == None
            {
                self.tags.insert(tag.clone(), vec!());
            }

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

        let file_entry = FileEntry::new(new_id, filename.clone(), tags.clone(), thumb_name.clone(), timestamp);
        self.files.insert(self.next_id, file_entry);

        self.next_id += 1;

        self.next_id - 1
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

    pub fn get_file_by_id(&self, id: usize) -> Option<FileEntry>
    {
        match self.files.get(&id){
            Some(file) => {
                Some(file.clone())
            },
            None => None
        }
    }
    
    /**
      Returns all files that have all the tags in the list
     */
    pub fn get_files_with_tags(&self, tags: Vec<String>) -> Vec<FileEntry>
    {
        let mut tags = tags.clone();

        if tags.len() == 0
        {
            return self.files.values().map(|x|{x.clone()}).collect();
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

    pub fn get_file_with_id(&self, id: usize) -> Option<&FileEntry>
    {
        self.files.get(&id)
    }

    pub fn get_files_with_tags_and_function(
                &self, 
                tags: Vec<String>,
                filters: &Vec<Box<Fn(&FileEntry) -> bool>>
            ) -> Vec<FileEntry>
    {
        let with_tags = self.get_files_with_tags(tags);

        with_tags.into_iter().filter(|x|{
            for pred in filters
            {
                if pred(x)
                {
                    return false;
                }
            }
            true
        }).collect()
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

    /**
     */
    pub fn add_file_to_db(&mut self, filename: &String, thumb_name: &String, tags: &Vec<String>, 
                            timestamp: u64) -> usize
    {
        //Save the file into the database
        self.db.add_new_file(&filename, thumb_name, tags, timestamp)
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

    pub fn get_saved_file_path(&self) -> String
    {
        self.file_path.clone()
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

        let id1 = fdb.add_new_file(
            &"test1".to_string(), 
            &"thumb1".to_string(),
            &vec!("tag1".to_string(), "tag2".to_string()),
            0);
        let id2 = fdb.add_new_file(
            &"test2".to_string(),
            &"thumb2".to_string(),
            &vec!("tag1".to_string(), "tag3".to_string()),
            0);

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);

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

        fdb.add_new_file(&"test1".to_string(), &"thumb1".to_string(), &vec!("common_tag".to_string(), "only1_tag".to_string()), 0);
        fdb.add_new_file(&"test2".to_string(), &"thumb2".to_string(), &vec!("common_tag".to_string(), "only2_3_tag".to_string()), 0);
        fdb.add_new_file(&"test3".to_string(), &"thumb3".to_string(), &vec!("common_tag".to_string(), "only2_3_tag".to_string()), 0);

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

        let no_tags = fdb.get_files_with_tags(vec!());
        assert!(no_tags.len() == 3);
    }

    fn timestamp_test()
    {
        let mut fdb = FileDatabase::new();

        let files = vec!(
            fdb.add_new_file(&String::from("1"), &String::from("1"), &vec!(), 0),
            fdb.add_new_file(&String::from("2"), &String::from("2"), &vec!(), 100),
            fdb.add_new_file(&String::from("3"), &String::from("3"), &vec!(), 150),
            fdb.add_new_file(&String::from("4"), &String::from("4"), &vec!(), 150),
            fdb.add_new_file(&String::from("5"), &String::from("5"), &vec!(), 50),
            fdb.add_new_file(&String::from("6"), &String::from("6"), &vec!(), 200)
        );

        let less_than_120 = Box::new(|x: &FileEntry|{x.timestamp < 120});
        let more_than_50 = Box::new(|x: &FileEntry|{x.timestamp < 120});
        let eq_0 = Box::new(|x: &FileEntry|{x.timestamp == 0});

        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(less_than_120)).len() == 3);
        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(eq_0)).len() == 1);

        let less_than_120 = Box::new(|x: &FileEntry|{x.timestamp < 120});
        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(less_than_120, more_than_50)).len() == 1);
    }
}

