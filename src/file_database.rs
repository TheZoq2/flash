
extern crate image;
extern crate rand;
extern crate rustc_serialize;

use std::vec::Vec;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use schema::{files};

use chrono::NaiveDateTime;

use iron::typemap::Key;


/**
  A reference to a file stored in the file database
 */
#[derive(Queryable, Identifiable, Associations, Clone, RustcEncodable)]
pub struct File
{
    //The unique ID of this file in the db
    pub id: i32,
    //The path to the actual file
    pub filename: String,

    pub thumbnail_path: String,

    pub creation_date: Option<NaiveDateTime>,

    pub is_uploaded: bool,

    pub tags: Vec<String>
}

#[derive(Insertable)]
#[table_name="files"]
pub struct NewFile<'a>
{
    filename: &'a str,
    thumbnail_path: &'a str,

    creation_date: NaiveDateTime,

    is_uploaded: bool,

    tags: Vec<String>
}

impl<'a> NewFile<'a>
{
    pub fn new(filename: &'a str, thumbnail_path: &'a str, creation_time: NaiveDateTime, tags: Vec<String>)
        -> NewFile<'a>
    {
        NewFile {
            filename,
            thumbnail_path,
            creation_date: creation_time,
            is_uploaded: false,
            tags
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
    //TODO: Handle errors when writing to the database
    pub fn add_new_file(&mut self,
                        filename: &str,
                        thumb_name: &str, 
                        tags: &Vec<String>,
                        timestamp: u64
                    ) -> File
    {
        let timestamp = timestamp as i64;
        let new_file = NewFile::new(
                filename,
                thumb_name,
                NaiveDateTime::from_timestamp(timestamp, 0),
                tags.clone()
            );

        let file: File = diesel::insert(&new_file).into(files::table)
            .get_result(&self.connection)
            .expect("Error saving new file");

        file
    }

    /**
      Changes the tags of a specified file. Returns the new file object
    */
    #[must_use]
    pub fn change_file_tags(&self, file: File, tags: &Vec<String>) -> Result<File, String>
    {
        let result = 
            diesel::update(files::table.find(file.id))
                .set(files::tags.eq(tags))
                .get_result(&self.connection);

        match result
        {
            Ok(val) => Ok(val),
            Err(e) => Err(format!("Failed to update file tags. {:?}", e))
        }
    }

    /**
      Returns all files that have all the tags in the list
     */
    pub fn get_files_with_tags(&self, tags: Vec<String>) -> Vec<File>
    {
        files::table.filter(files::tags.contains(tags))
            .get_results(&self.connection)
            .expect("Error retrieving photos with tags")
    }

    pub fn get_file_paths_with_tags(&self, tags: Vec<String>) -> Vec<String>
    {
        self.get_files_with_tags(tags).iter().map(|x|{x.filename.clone()}).collect()
    }

    pub fn get_file_with_id(&self, id: i32) -> Option<File>
    {
        let result =
            files::table.find(id).get_result::<File>(&self.connection);

        match result
        {
            Ok(val) => Some(val),
            Err(_) => None
        }
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
pub fn get_file_paths_from_files(files: Vec<File>) -> Vec<String>
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
        diesel::delete(schema::files::table).execute(&connection).unwrap();

        let fdb = FileDatabase::new(establish_connection(), String::from("/tmp/flash"));

        assert_eq!(fdb.get_file_amount(), 0);

        fdb
    }

    /**
      Since the results of these tests depend on the database state,
      they can not be run concurrently
    */
    #[test]
    fn database_test()
    {
        add_test();
        multi_tag_test();
        modify_tags_test();
    }

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
        assert!(fdb.get_file_paths_with_tags(vec!("tag1".to_string())).contains(&"test1".to_string()));
        assert!(fdb.get_file_paths_with_tags(vec!("tag1".to_string())).contains(&"test2".to_string()));

        //Ensure only the correct tags are found when searching for the other tags
        assert!(fdb.get_file_paths_with_tags(vec!("tag2".to_string())).contains(&"test1".to_string()));
        assert!(fdb.get_file_paths_with_tags(vec!("tag2".to_string())).contains(&"test2".to_string()) == false);

        assert!(fdb.get_file_paths_with_tags(vec!("tag3".to_string())).contains(&"test2".to_string()));
        assert!(fdb.get_file_paths_with_tags(vec!("tag3".to_string())).contains(&"test1".to_string()) == false);

        //Ensure that tags that don't exist don't return anything
        assert!(fdb.get_file_paths_with_tags(vec!("unused_tag".to_string())).is_empty());
    }

    fn multi_tag_test()
    {
        let mut fdb = get_file_database();

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

    fn modify_tags_test()
    {
        let mut fdb = get_file_database();

        let id = fdb.add_new_file(&"test1".to_string(), &"thumb1".to_string(), &vec!("old_tag".to_string()), 0);

        fdb.change_file_tags(id, &vec!("new_tag".to_string())).unwrap();

        assert!(fdb.get_files_with_tags(vec!("new_tag".to_string())).len() == 1);
        assert!(fdb.get_files_with_tags(vec!("old_tag".to_string())).len() == 0);
    }

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

        let less_than_120 = |x: &File|{x.timestamp < 120};
        let more_than_50 = |x: &File|{x.timestamp < 120};
        let eq_0 = |x: &File|{x.timestamp == 0};

        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&less_than_120)).len() == 3);
        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&eq_0)).len() == 1);

        assert!(fdb.get_files_with_tags_and_function(vec!(), &vec!(&less_than_120, &more_than_50)).len() == 1);
    }
    */
}

