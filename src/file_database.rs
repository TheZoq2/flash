use std::vec::Vec;
use std::collections::HashMap;

use rustc_serialize::json;

use std::io::prelude::*;
use std::io;
use std::fs::{File, OpenOptions};

use settings::Settings;

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

#[derive(RustcEncodable, RustcDecodable)]
pub struct FileDatabase
{
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

            self.tags.get_mut(tag).unwrap().push(new_id);
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
      Returns paths to all file objects that are part of a tag
     */
    pub fn get_file_paths_with_tag(&self, tag: String) -> Vec<String>
    {
        let mut result = Vec::<String>::new();
        for file in &self.get_files_with_tag(tag)
        {
            result.push(file.path.clone());
        }

        result
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

    pub fn get_mut_db(&mut self) -> &mut FileDatabase
    {
        &mut self.db
    }

    pub fn save(&self) -> Result<(), io::Error>
    {
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&self.db_path){
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
}
