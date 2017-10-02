
extern crate image;
extern crate rand;

use std::vec::Vec;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::expression::{not, any};

use schema::files;

use chrono::NaiveDateTime;

use iron::typemap::Key;

use std::path::PathBuf;

use search;
use date_search;

/**
  A reference to a file stored in the file database
 */
#[derive(Queryable, Identifiable, Associations, Clone, PartialEq, Debug)]
pub struct File {
    // The unique ID of this file in the db
    pub id: i32,
    // The path to the actual file
    pub filename: String,

    pub thumbnail_path: String,

    pub creation_date: Option<NaiveDateTime>,

    pub is_uploaded: bool,

    pub tags: Vec<String>,
}

#[derive(Insertable)]
#[table_name = "files"]
pub struct NewFile<'a> {
    filename: &'a str,
    thumbnail_path: &'a str,

    creation_date: NaiveDateTime,

    is_uploaded: bool,

    tags: Vec<String>,
}

impl<'a> NewFile<'a> {
    pub fn new(
        filename: &'a str,
        thumbnail_path: &'a str,
        creation_time: NaiveDateTime,
        tags: Vec<String>,
    ) -> NewFile<'a> {
        NewFile {
            filename,
            thumbnail_path,
            creation_date: creation_time,
            is_uploaded: false,
            tags,
        }
    }
}


pub struct FileDatabase {
    connection: PgConnection,
    file_save_path: PathBuf,
}
impl Key for FileDatabase {
    type Value = FileDatabase;
}

impl FileDatabase {
    pub fn new(connection: PgConnection, file_save_path: PathBuf) -> FileDatabase {
        // If the destination folder does not exist, it should be created
        FileDatabase {
            connection,
            file_save_path,
        }
    }

    /**
      Adds a new file entry to the "database". It is given a new unique ID and the
      file is added to the tags which it should be part of. If some of those tags don't
      exist yet, then they are added

      Returns a `File` struct of the added image
     */
    //TODO: Handle errors when writing to the database
    pub fn add_new_file(
            &mut self,
            filename: &str,
            thumb_name: &str,
            tags: &[String],
            timestamp: u64,
        ) -> File
    {
        let timestamp = timestamp as i64;
        let new_file = NewFile::new(
            filename,
            thumb_name,
            NaiveDateTime::from_timestamp(timestamp, 0),
            tags.to_owned(),
        );

        let file: File = diesel::insert(&new_file)
            .into(files::table)
            .get_result(&self.connection)
            .expect("Error saving new file");

        file
    }

    /**
      Changes the tags of a specified file. Returns the new file object
    */
    #[must_use]
    pub fn change_file_tags(&self, file: &File, tags: &[String]) -> Result<File, String> {
        let result = diesel::update(files::table.find(file.id))
            .set(files::tags.eq(tags))
            .get_result(&self.connection);

        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(format!("Failed to update file tags. {:?}", e)),
        }
    }

    /**
      Returns all files that have all the tags in the list and that dont have any
      tags in the negated tag list
     */
    pub fn search_files(&self, query: search::SavedSearchQuery) -> Vec<File> {
        let search::SavedSearchQuery{tags, negated_tags, date_constraints} = query;

        // Construct the database query
        // construct static query parameters
        let mut db_query = files::table.into_boxed()
            .filter(files::tags.contains(tags));

        if negated_tags.len() != 0 {
            db_query = db_query.filter(not(files::tags.contains(negated_tags)));
        }

        // Add dynamic parts of the query
        for interval in &date_constraints.intervals {
            db_query = db_query.filter(
                    files::creation_date.between(interval.start..interval.end)
                );
        }

        // Execute the database query and filter things that can't be filtered using sql
        let db_result = db_query.load(&self.connection).expect("Error executing database query");

        db_result.into_iter()
            .filter(|file: &File| {
                date_constraints.constraints
                    .iter()
                    .fold(true, |acc, constraint_function|{
                        if let Some(creation_date) = file.creation_date {
                            acc && constraint_function(&creation_date)
                        }
                        else {false}
                    })
            })
            .collect()
    }

    pub fn get_file_with_id(&self, id: i32) -> Option<File> {
        let result = files::table.find(id).get_result::<File>(&self.connection);

        match result {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }


    /**
      Returns the path to the folder where files should be stored

      TODO: Move out of database code
    */
    pub fn get_file_save_path(&self) -> PathBuf {
        self.file_save_path.clone()
    }

    fn get_file_amount(&self) -> i64 {
        use schema::files::dsl::*;

        files.count().get_result(&self.connection).unwrap()
    }

    #[cfg(test)]
    pub fn reset(&self) {
        diesel::delete(files::table)
            .execute(&self.connection)
            .unwrap();
    }
}

/**
 * Returns a vector of paths from a vector of file entrys
 */
#[cfg(test)]
pub fn get_file_paths_from_files(files: &[File]) -> Vec<String> {
    let mut result = vec![];

    for file in files {
        result.push(file.filename.clone());
    }

    result
}


#[cfg(test)]
pub mod db_test_helpers {
    use file_database::*;

    use std::sync::Mutex;
    use std::sync::Arc;

    use dotenv::dotenv;
    use std::env;

    use diesel::pg::PgConnection;

    use std::fs;
    use std::io;

    //Establish a connection to the postgres database
    fn establish_connection() -> PgConnection {
        dotenv().ok();

        let database_url = env::var("DATABASE_TEST_URL")
            .expect("DATABASE_TEST_URL must be set. Perhaps .env is missing?");
        PgConnection::establish(&database_url)
            .expect(&format!("Error connecting to {}", database_url))
    }

    pub fn get_test_storage_path() -> String {
        dotenv().ok();
        env::var("TEST_FILE_STORAGE_PATH").expect(
            "TEST_FILE_STORAGE_PATH must be set. Perhaps .env is missing?",
        )
    }

    fn create_db() -> FileDatabase {
        let test_file_storage_path = get_test_storage_path();

        match fs::create_dir(test_file_storage_path.clone()) {
            Ok(_) => {}
            Err(e) => {
                if e.kind() != io::ErrorKind::AlreadyExists {
                    panic!("{:?}", e)
                }
            }
        };

        let fdb = FileDatabase::new(
            establish_connection(),
            PathBuf::from(test_file_storage_path),
        );

        fdb
    }

    lazy_static! {
        // Most functions that modify the database already want
        // `Arc<Mutex<fdb>>` so it has to have two layers of mutex
        static ref FDB: Arc<Mutex<Arc<Mutex<FileDatabase>>>>
                = Arc::new(Mutex::new(Arc::new(Mutex::new(create_db()))));
    }

    pub fn get_database() -> Arc<Mutex<Arc<Mutex<FileDatabase>>>> {
        FDB.clone()
    }

    pub fn run_test<F: Fn(&mut FileDatabase)>(test: F) {
        let fdb = FDB.lock().unwrap();
        let mut fdb = fdb.lock().unwrap();
        fdb.reset();
        assert_eq!(fdb.get_file_amount(), 0);

        test(&mut fdb);
    }
}

/*
   Tests
 */
#[cfg(test)]
mod db_tests {
    use file_database::*;

    //////////////////////////////////////////////////
    // Helper functions
    //////////////////////////////////////////////////
    fn get_files_with_tags(fdb: &FileDatabase, tags: Vec<String>, negated: Vec<String>)
        -> Vec<File>
    {
        let file_query = search::SavedSearchQuery::with_tags((tags, negated));

        fdb.search_files(file_query)
    }

    fn get_file_paths_with_tags(fdb: &FileDatabase, tags: Vec<String>, negated: Vec<String>) 
        -> Vec<String>
    {
        get_files_with_tags(fdb, tags, negated)
            .iter()
            .map(|file| file.filename.clone())
            .collect()
    }

    //////////////////////////////////////////////////
    // Tests
    //////////////////////////////////////////////////

    /**
      Since the results of these tests depend on the database state,
      they can not be run concurrently
    */
    #[test]
    fn database_test() {
        db_test_helpers::run_test(add_test);
        db_test_helpers::run_test(multi_tag_test);
        db_test_helpers::run_test(modify_tags_test);
        db_test_helpers::run_test(negated_tags_test);
    }

    fn add_test(fdb: &mut FileDatabase) {
        fdb.add_new_file(
            &"test1".to_string(),
            &"thumb1".to_string(),
            &vec!["tag1".to_string(), "tag2".to_string()],
            0,
        );
        fdb.add_new_file(
            &"test2".to_string(),
            &"thumb2".to_string(),
            &vec!["tag1".to_string(), "tag3".to_string()],
            0,
        );

        assert_eq!(fdb.get_file_amount(), 2);

        //Ensure both files are found when searching for tag1
        let only_tag_1 = get_file_paths_with_tags(&fdb, vec!["tag1".to_string()], vec![]);
        assert!(
            only_tag_1.contains(&"test1".to_string())
        );
        assert!(
            get_file_paths_with_tags(&fdb, vec!["tag1".to_string()], vec![])
                .contains(&"test2".to_string())
        );

        //Ensure only the correct tags are found when searching for the other tags
        assert!(
            get_file_paths_with_tags(&fdb, vec!["tag2".to_string()], vec![])
                .contains(&"test1".to_string())
        );
        assert!(
            get_file_paths_with_tags(&fdb, vec!["tag2".to_string()], vec![])
                .contains(&"test2".to_string()) == false
        );

        assert!(
            get_file_paths_with_tags(&fdb, vec!["tag3".to_string()], vec![])
                .contains(&"test2".to_string())
        );
        assert!(
            get_file_paths_with_tags(&fdb, vec!["tag3".to_string()], vec![])
                .contains(&"test1".to_string()) == false
        );

        //Ensure that tags that don't exist don't return anything
        assert!(
            get_file_paths_with_tags(&fdb, vec!["unused_tag".to_string()], vec![])
                .is_empty()
        );
    }

    fn multi_tag_test(fdb: &mut FileDatabase) {
        fdb.add_new_file(
            "test1",
            "thumb1",
            &vec!["common_tag".to_string(), "only1_tag".to_string()],
            0,
        );
        fdb.add_new_file(
            "test2",
            "thumb2",
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
        );
        fdb.add_new_file(
            "test3",
            "thumb3",
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
        );

        let common_2_3 = get_files_with_tags(
            &fdb,
            vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            vec![],
        );
        assert!(get_file_paths_from_files(&common_2_3).contains(&"test1".to_owned()) == false);
        assert!(get_file_paths_from_files(&common_2_3).contains(&"test2".to_owned()));
        assert!(get_file_paths_from_files(&common_2_3).contains(&"test3".to_owned()));

        let common_1 = get_files_with_tags(&fdb, vec!["common_tag".to_string()], vec![]);
        assert!(get_file_paths_from_files(&common_1).contains(&"test1".to_owned()));
        assert!(get_file_paths_from_files(&common_1).contains(&"test2".to_owned()));
        assert!(get_file_paths_from_files(&common_1).contains(&"test3".to_owned()));

        let only_1 = get_files_with_tags(&fdb, vec!["only1_tag".to_string()], vec![]);
        assert!(get_file_paths_from_files(&only_1).contains(&"test1".to_owned()));
        assert!(get_file_paths_from_files(&only_1).contains(&"test2".to_owned()) == false);
        assert!(get_file_paths_from_files(&only_1).contains(&"test3".to_owned()) == false);

        let no_tags = get_files_with_tags(&fdb, vec![], vec![]);
        assert!(no_tags.len() == 3);
    }

    fn modify_tags_test(fdb: &mut FileDatabase) {
        let file = fdb.add_new_file("test1", "thumb1", &vec!["old_tag".to_string()], 0);

        fdb.change_file_tags(&file, &vec!["new_tag".to_string()])
            .unwrap();

        assert!(
            get_files_with_tags(&fdb, vec!["new_tag".to_string()], vec![])
                .len() == 1
        );
        assert!(
            get_files_with_tags(&fdb, vec!["old_tag".to_string()], vec![])
                .len() == 0
        );
    }


    fn negated_tags_test(fdb: &mut FileDatabase) {
        fdb.add_new_file(
            &"test1".to_string(),
            &"thumb1".to_string(),
            &vec!["common_tag".to_string(), "only1_tag".to_string()],
            0,
        );
        fdb.add_new_file(
            &"test2".to_string(),
            &"thumb2".to_string(),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
        );
        fdb.add_new_file(
            &"test3".to_string(),
            &"thumb3".to_string(),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
        );

        let result = get_files_with_tags(
            &fdb,
            vec!["common_tag".to_string()],
            vec!["only1_tag".to_string()],
        );
        assert!(get_file_paths_from_files(&result).contains(&"test1".to_owned()) == false);
        assert!(get_file_paths_from_files(&result).contains(&"test2".to_owned()));
        assert!(get_file_paths_from_files(&result).contains(&"test3".to_owned()));
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
