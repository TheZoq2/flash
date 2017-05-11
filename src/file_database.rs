
extern crate image;
extern crate rand;

use std::vec::Vec;
use std::collections::HashMap;

use rustc_serialize::json;

use std::io::prelude::*;
use std::fs::{File};

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use schema;
use schema::files;

use chrono;
use chrono::naive::time::NaiveDateTime;

/**
  A reference to a file stored in the file database
 */
#[derive(Queryable, Clone)]
pub struct FileEntry
{
    //The unique ID of this file in the db
    id: i32,
    //The path to the actual file
    pub path: String,

    pub timestamp: NaiveDateTime,

    pub thumbnail_path: String,

    is_uploaded: bool
}

impl FileEntry
{
    pub fn has_tag(&self, tag: String) -> bool
    {
        self.tags.contains(&tag)
    }
}

#[derive(Insertable)]
#[table_name="files"]
pub struct NewFileEntry<'a>
{
    filename: &'a str,
    thumbnail_path: &'a str,

    timestamp: NaiveDateTime,

    is_uploaded: bool
}

impl<'a> NewFileEntry<'a>
{
    pub fn new(filename: &'a str, thumbnail_path: &'a str, creation_time: NaiveDateTime) 
        -> NewFileEntry<'a>
    {
        NewFileEntry {
            filename,
            thumbnail_path,
            timestamp: creation_time,
            is_uploaded: false
        }
    }
}


pub struct FileDatabase
{
    connection: PgConnection
}

impl FileDatabase
{
    pub fn new(connection: PgConnection) -> FileDatabase
    {
        FileDatabase{
            connection
        }
    }

    /**
      Adds a new file entry to the "database". It is given a new unique ID and the
      file is added to the tags which it should be part of. If some of those tags don't 
      exist yet, then they are added

      Returns the ID of the added image
     */
    //TODO: Handle tags and time
    pub fn add_new_file(&mut self,
                        filename: &str,
                        thumb_name: &str, 
                        tags: &Vec<String>,
                        timestamp: u64
                    ) -> FileEntry
    {
        use schema::files;

        let new_file = NewFileEntry::new(filename, thumb_name, NaiveDateTime::from_timestamp(timestamp, 0));

        diesel::insert(&new_file).into(files::table)
            .get_result(&self.connection)
            .expect("Error saving new file")
    }

    #[must_use]
    pub fn change_file_tags(&mut self, id: usize, tags: &Vec<String>) -> Result<(), String>
    {
        //First we need to find out what tags the file had before. 
        //We can not borrow the file object here because we are going to be
        //modifying self later on
        let file = match self.get_file_by_id(id)
        {
            Some(file) => file,
            None =>
            {
                return Err(String::from(format!("Failed to modify file, ID {} doesn't exist", id)));
            }
        };

        self.remove_tags_of_file(id, &file.tags);

        self.set_file_tags(id, tags);



        //This time we can just unwrap the value because we know that
        //the last get_file_by_id operation succeded if we got here
        let file = &mut self.get_mut_file_by_id(id).unwrap();

        file.tags = tags.clone();

        Ok(())
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

    fn get_mut_file_by_id(&mut self, id: usize) -> Option<&mut FileEntry>
    {
        match self.files.get_mut(&id){
            Some(file) => {
                Some(file)
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

    pub fn get_file_with_id(&self, id: usize) -> Option<&FileEntry>
    {
        self.files.get(&id)
    }

    pub fn get_files_with_tags_and_function(
                &self, 
                tags: Vec<String>,
                filters: &Vec<&Fn(&FileEntry) -> bool>
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

    fn remove_tags_of_file(&mut self, file_id: usize, tags: &Vec<String>)
    {
        for tag in tags
        {
            let id_list = &mut self.tags.get_mut(tag).unwrap();

            let mut index = 0;
            for i in 0..id_list.len()
            {
                if id_list[i] == file_id
                {
                    index = i;
                    break
                }
            }

            id_list.swap_remove(index);
        }
    }

    /**
      Sets the tags of a file without removing old tags from it. This is dangerous
      since it can cause the db tag list to desync with the file entry. Make sure you
      only run this when you are sure that the file has no previous tags stored
     */
    fn set_file_tags(&mut self, file: usize, tags: &Vec<String>)
    {
        //Add all the tags to the map
        for tag in tags
        {
            if self.tags.get(tag) == None
            {
                self.tags.insert(tag.clone(), vec!());
            }

            //Insert the new image using the binary search function.
            let vec = self.tags.get_mut(tag).unwrap();
            match vec.binary_search(&file){
                Ok(_) => {
                    //This shouldn't happen
                    println!("ID {} is already part of tag {}. This is an error in FileDatabase::add_new_file", file, tag);
                },
                //If binary search returns Err, it means it didn't find an element and it returns
                //the index where the element would be
                Err(new_index) => {vec.insert(new_index, file)}
            }
        }
    }
}

/**
 * Returns a vector of paths from a vector of file entrys
 */
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

    #[test]
    fn modify_tags_test()
    {
        let mut fdb = FileDatabase::new();

        let id = fdb.add_new_file(&"test1".to_string(), &"thumb1".to_string(), &vec!("old_tag".to_string()), 0);

        fdb.change_file_tags(id, &vec!("new_tag".to_string())).unwrap();

        assert!(fdb.get_files_with_tags(vec!("new_tag".to_string())).len() == 1);
        assert!(fdb.get_files_with_tags(vec!("old_tag".to_string())).len() == 0);
    }

    fn timestamp_test()
    {
        let mut fdb = FileDatabase::new();

        fdb.add_new_file(&String::from("1"), &String::from("1"), &vec!(), 0);
        fdb.add_new_file(&String::from("2"), &String::from("2"), &vec!(), 100);
        fdb.add_new_file(&String::from("3"), &String::from("3"), &vec!(), 150);
        fdb.add_new_file(&String::from("4"), &String::from("4"), &vec!(), 150);
        fdb.add_new_file(&String::from("5"), &String::from("5"), &vec!(), 50);
        fdb.add_new_file(&String::from("6"), &String::from("6"), &vec!(), 200);

        let less_than_120 = |x: &FileEntry|{x.timestamp < 120};
        let more_than_50 = |x: &FileEntry|{x.timestamp < 120};
        let eq_0 = |x: &FileEntry|{x.timestamp == 0};

        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&less_than_120)).len() == 3);
        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&eq_0)).len() == 1);

        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&less_than_120, &more_than_50)).len() == 1);
    }
}

