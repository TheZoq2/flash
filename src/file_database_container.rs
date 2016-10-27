extern crate image;
extern crate rand;

use std::vec::Vec;

use rustc_serialize::json;

use std::io::prelude::*;
use std::io;
use std::fs::{OpenOptions};

use settings::Settings;

use iron::typemap::Key;

use file_database::FileDatabase;

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

    #[must_use]
    pub fn change_file_tags(&mut self, id: usize, tags: &Vec<String>) -> Result<(), String>
    {
        self.db.change_file_tags(id, tags)
    }
}
