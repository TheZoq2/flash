
extern crate image;
extern crate rand;
extern crate rustc_serialize;

use std::vec::Vec;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use schema;
use schema::files;

use chrono::NaiveDateTime;

use iron::typemap::Key;


/**
  A tag stored in the database. Files can be linked to tags through
  tag_links
*/
//#[derive(Queryable, Identifiable, Associations)]
//#[has_many(tag_links)]
//struct Tag
//{
//    pub id: i32,
//    pub name: String
//}
//
//#[derive(Insertable)]
//#[table_name="tags"]
//struct NewTag<'a>
//{
//    text: &'a str
//}
//
//
///**
//  A link between a file and a tag
//*/
//#[derive(Queryable, Identifiable, Associations)]
////#[belongs_to(tags), foreign_key="tag_id"]
////#[belongs_to(files), foreign_key="file_id"]
//#[belongs_to(tags)]
//#[belongs_to(files)]
//pub struct TagLink
//{
//    id: i32,
//    file_id: i32,
//    tag_id: i32
//}
//
//#[derive(Insertable)]
//#[table_name="tag_links"]
//struct NewTagLink
//{
//    pub file_id: i32,
//    pub tag_id: i32
//}



/**
  A reference to a file stored in the file database
 */
#[derive(Queryable, Clone, RustcEncodable)]
pub struct FileEntry
{
    //The unique ID of this file in the db
    pub id: i32,
    //The path to the actual file
    pub filename: String,

    pub thumbnail_path: String,

    pub creation_date: Option<NaiveDateTime>,

    is_uploaded: bool
}

#[derive(Insertable)]
#[table_name="files"]
pub struct NewFileEntry<'a>
{
    filename: &'a str,
    thumbnail_path: &'a str,

    creation_date: NaiveDateTime,

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
            creation_date: creation_time,
            is_uploaded: false
        }
    }
}


pub struct FileDatabase
{
    connection: PgConnection,
    file_save_path: String
}
impl Key for FileDatabase { type Value = FileDatabase; }

impl FileDatabase
{
    pub fn new(connection: PgConnection, file_save_path: String) -> FileDatabase
    {
        FileDatabase{
            connection,
            file_save_path
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

        let timestamp = timestamp as i64;
        let new_file = NewFileEntry::new(filename, thumb_name, NaiveDateTime::from_timestamp(timestamp, 0));

        diesel::insert(&new_file).into(files::table)
            .get_result(&self.connection)
            .expect("Error saving new file")
    }

    #[must_use]
    pub fn change_file_tags(&mut self, id: i32, tags: &Vec<String>) -> Result<(), String>
    {
        unimplemented!()
    }




    /**
      Returns all FileEntry objects which are part of a specific tag
     */
    pub fn get_files_with_tag(&self, tag: String) -> Vec<FileEntry>
    {
        unimplemented!();
    }

    pub fn get_file_by_id(&self, id: i32) -> Option<FileEntry>
    {
        unimplemented!();
    }

    fn get_mut_file_by_id(&mut self, id: i32) -> Option<&mut FileEntry>
    {
        unimplemented!();
    }
    
    /**
      Returns all files that have all the tags in the list
     */
    pub fn get_files_with_tags(&self, tags: Vec<String>) -> Vec<FileEntry>
    {
        unimplemented!();
    }

    /**
      Returns paths to all file objects that are part of a tag
     */
    pub fn get_file_paths_with_tag(&self, tag: String) -> Vec<String>
    {
        unimplemented!();
    }
    pub fn get_file_paths_with_tags(&self, tag: Vec<String>) -> Vec<String>
    {
        unimplemented!();
    }

    pub fn get_file_with_id(&self, id: i32) -> Option<&FileEntry>
    {
        unimplemented!()
    }

    pub fn get_files_with_tags_and_function(
                &self, 
                tags: Vec<String>,
                filters: &Vec<&Fn(&FileEntry) -> bool>
            ) -> Vec<FileEntry>
    {
        unimplemented!();
    }

    fn remove_tags_of_file(&mut self, file_id: i32, tags: &Vec<String>)
    {
        unimplemented!();
    }

    /**
      Sets the tags of a file without removing old tags from it. This is dangerous
      since it can cause the db tag list to desync with the file entry. Make sure you
      only run this when you are sure that the file has no previous tags stored
     */
    fn set_file_tags(&mut self, file: i32, tags: &Vec<String>)
    {
        unimplemented!();
    }

    /**
      Returns the path to the folder where files should be stored

      TODO: Move out of database code
    */
    pub fn get_file_save_path(&self) -> String
    {
        return self.file_save_path.clone();
    }

    fn get_file_amount(&self) -> i64
    {
        use schema::files::dsl::*;

        files.count().get_result(&self.connection).unwrap()
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
        result.push(file.filename.clone());
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

    use dotenv::dotenv;
    use std::env;

    use diesel;
    use schema;

    use diesel::prelude::*;
    use diesel::pg::PgConnection;

    //Establish a connection to the postgres database
    fn establish_connection() -> PgConnection
    {
        dotenv().ok();

        let database_url = env::var("DATABASE_TEST_URL")
            .expect("DATABASE_TEST_URL must be set. Perhaps .env is missing?");
        PgConnection::establish(&database_url)
            .expect(&format!("Error connecting to {}", database_url))
    }

    fn get_file_database() -> FileDatabase
    {
        let connection = establish_connection();

        //Clear the tables
        diesel::delete(schema::files::table).execute(&connection);

        let fdb = FileDatabase::new(establish_connection(), String::from("/tmp/flash"));

        assert_eq!(fdb.get_file_amount(), 0);

        fdb
    }

    #[test]
    fn add_test()
    {
        let mut fdb = get_file_database();

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

        assert_eq!(fdb.get_file_amount(), 2);

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

    /*
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
    */

    /*
    #[test]
    fn modify_tags_test()
    {
        let mut fdb = FileDatabase::new();

        let id = fdb.add_new_file(&"test1".to_string(), &"thumb1".to_string(), &vec!("old_tag".to_string()), 0);

        fdb.change_file_tags(id, &vec!("new_tag".to_string())).unwrap();

        assert!(fdb.get_files_with_tags(vec!("new_tag".to_string())).len() == 1);
        assert!(fdb.get_files_with_tags(vec!("old_tag".to_string())).len() == 0);
    }
    */

    /*
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
    */
}

