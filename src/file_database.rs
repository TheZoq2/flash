
extern crate image;
extern crate rand;

use std::vec::Vec;

use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::expression::{not};

use schema::{files, syncpoints, changes};

use chrono::NaiveDateTime;


use std::path::PathBuf;

use search;
use error::{Result};
use changelog::{
    Change,
    ChangeDbEntry,
    SyncPoint,
    InsertableChange,
    ChangeCreationPolicy,
    ChangeType,
    UpdateType,
    sorted_changes
};


/**
  A reference to a file stored in the file database
 */
#[derive(Queryable, Identifiable, Associations, AsChangeset, Clone, PartialEq, Debug)]
pub struct File {
    // The unique ID of this file in the db
    pub id: i32,
    // The path to the actual file
    pub filename: String,

    pub thumbnail_path: Option<String>,

    pub creation_date: NaiveDateTime,

    pub is_uploaded: bool,

    pub tags: Vec<String>,
}

#[derive(Insertable)]
#[table_name = "files"]
pub struct NewFile<'a> {
    id: i32,

    filename: &'a str,
    thumbnail_path: Option<&'a str>,

    creation_date: NaiveDateTime,

    is_uploaded: bool,

    tags: Vec<String>,
}

impl<'a> NewFile<'a> {
    pub fn new(
        id: i32,
        filename: &'a str,
        thumbnail_path: Option<&'a str>,
        creation_date: NaiveDateTime,
        tags: Vec<String>,
    ) -> NewFile<'a> {
        NewFile {
            id,
            filename,
            thumbnail_path,
            creation_date,
            is_uploaded: false,
            tags,
        }
    }
}


pub struct FileDatabase {
    connection: PgConnection,
    file_save_path: PathBuf,
}

impl FileDatabase {
    pub fn new(url: &str, file_save_path: PathBuf) -> Result<FileDatabase> {
        let connection = PgConnection::establish(&url)?;

        // If the destination folder does not exist, it should be created
        Ok(FileDatabase {
            connection,
            file_save_path,
        })
    }

    /**
      Adds a new file entry to the "database". It is given a new unique ID and the
      file is added to the tags which it should be part of. If some of those tags don't
      exist yet, then they are added

      Returns a `File` struct of the added image
     */
    //TODO: Handle errors when writing to the database
    pub fn add_new_file(
            &self,
            id: i32,
            filename: &str,
            thumb_name: Option<&str>,
            tags: &[String],
            timestamp: u64,
            change_policy: ChangeCreationPolicy
        ) -> File
    {
        let timestamp = timestamp as i64;
        let new_file = NewFile::new(
            id,
            filename,
            thumb_name,
            NaiveDateTime::from_timestamp(timestamp, 0),
            tags.to_owned(),
        );

        let file: File = diesel::insert(&new_file)
            .into(files::table)
            .get_result(&self.connection)
            .expect("Error saving new file");

        self.handle_change_creation_policy(&change_policy, id, ChangeType::FileAdded).unwrap();
        for tag in tags {
            self.handle_change_creation_policy(
                &change_policy,
                id,
                ChangeType::Update(UpdateType::TagAdded(tag.to_owned()))
            ).unwrap();
        }

        file
    }

    /**
      Changes the tags of a specified file. Returns the new file object
    */
    pub fn change_file_tags(
        &self,
        file: &File,
        tags: &[String],
        change_policy: ChangeCreationPolicy
    ) -> Result<File> {
        let result = diesel::update(files::table.find(file.id))
            .set(files::tags.eq(tags))
            .get_result(&self.connection);

        if let ChangeCreationPolicy::Yes(timestamp) = change_policy {
            // Filter removed tags and add changes for them
            let removed_tags =
                file.tags.iter()
                    .filter(|tag| !tags.contains(tag))
                    .collect::<Vec<_>>();

            let added_tags =
                tags.iter()
                    .filter(|tag| !file.tags.contains(tag))
                    .collect::<Vec<_>>();

            for tag in removed_tags {
                self.add_change(&Change::new(
                        timestamp,
                        file.id,
                        ChangeType::Update(UpdateType::TagRemoved(tag.to_string()))
                    ))?;
            }
            for tag in added_tags {
                self.add_change(&Change::new(
                        timestamp,
                        file.id,
                        ChangeType::Update(UpdateType::TagAdded(tag.to_string()))
                    ))?;
            }
        }

        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(e.into())
        }
    }

    pub fn update_file_without_creating_change(&self, file: &File) -> Result<File> {
        Ok(diesel::update(files::table.find(file.id))
            .set(file)
            .get_result(&self.connection)?
        )
    }
    pub fn drop_file(&self, file_id: i32, change_policy: ChangeCreationPolicy) -> Result<()> {
        diesel::delete(files::table.find(file_id))
            .execute(&self.connection)?;

        self.handle_change_creation_policy(&change_policy, file_id, ChangeType::FileRemoved)?;

        Ok(())
    }

    pub fn add_change(&self, change: &Change) -> Result<()> {
        diesel::insert(&InsertableChange::from(&ChangeDbEntry::from(change)))
            .into(changes::table)
            .execute(&self.connection)?;

        Ok(())
    }

    pub fn set_file_timestamp(
        &self,
        file: &File,
        timestamp: NaiveDateTime,
        change_policy: ChangeCreationPolicy
    ) -> Result<()> {
        let result = diesel::update(files::table.find(file.id))
            .set(files::creation_date.eq(timestamp))
            .execute(&self.connection);

        self.handle_change_creation_policy(
            &change_policy,
            file.id,
            ChangeType::Update(UpdateType::CreationDateChanged(timestamp))
        )?;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
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

        if !negated_tags.is_empty() {
            db_query = db_query.filter(not(files::tags.contains(negated_tags)));
        }

        // Add dynamic parts of the query
        for interval in &date_constraints.intervals {
            db_query = db_query.filter(
                    files::creation_date.between(interval.start..interval.end)
                );
        }

        db_query = db_query.order(files::creation_date);

        // Execute the database query and filter things that can't be filtered using sql
        let db_result = db_query.load(&self.connection).expect("Error executing database query");

        db_result.into_iter()
            .filter(|file: &File| {
                date_constraints.constraints
                    .iter()
                    .fold(true, |acc, constraint_function|{
                        acc && constraint_function(&file.creation_date)
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

    pub fn get_file_with_id_result(&self, id: i32) -> Result<File> {
        let result = files::table.find(id).get_result::<File>(&self.connection);

        match result {
            Ok(val) => Ok(val),
            Err(e) => Err(e.into())
        }
    }

    pub fn get_syncpoints(&self) -> Result<Vec<SyncPoint>> {
        Ok(syncpoints::table
            .select(syncpoints::last_change)
            .load(&self.connection)?
            .into_iter()
            .map(|last_change| SyncPoint{last_change})
            .collect()
        )
    }

    pub fn get_changes_after_timestamp(&self, timestamp: &NaiveDateTime) 
        -> Result<Vec<Change>>
    {
        Ok(Self::changes_from_db_entries(
            &changes::table
                .filter(changes::timestamp.gt(timestamp))
                .get_results(&self.connection)?
        )?)
    }
    pub fn get_all_changes(&self) -> Result<Vec<Change>>{
        Ok(Self::changes_from_db_entries(
            &changes::table
                .get_results(&self.connection)?
        )?)
    }

    fn changes_from_db_entries(db_entries: &[ChangeDbEntry]) -> Result<Vec<Change>> {
        let mut changes = vec!();
        for db_entry in db_entries {
            changes.push(Change::from_db_entry(&db_entry)?);
        }
        Ok(sorted_changes(&changes))
    }

    pub fn add_syncpoint(&self, syncpoint: &SyncPoint) -> Result<()> {
        diesel::insert(syncpoint)
            .into(syncpoints::table)
            .execute(&self.connection)?;

        Ok(())
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

    fn handle_change_creation_policy(
            &self,
            policy: &ChangeCreationPolicy,
            affected_file: i32,
            change_type: ChangeType,
        ) -> Result<()> {
        if let ChangeCreationPolicy::Yes(date) = *policy {
            self.add_change(&Change::new(date, affected_file, change_type))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub fn reset(&self) {
        diesel::delete(files::table)
            .execute(&self.connection)
            .unwrap();
        diesel::delete(changes::table)
            .execute(&self.connection)
            .unwrap();
        diesel::delete(syncpoints::table)
            .execute(&self.connection)
            .unwrap();
    }
}


#[cfg(test)]
pub mod db_test_helpers {
    use file_database::*;

    use std::sync::Mutex;
    use std::sync::Arc;

    use dotenv::dotenv;
    use std::env;

    use std::fs;
    use std::io;

    //Establish a connection to the postgres database
    fn db_url() -> String {
        dotenv().ok();

        env::var("DATABASE_TEST_URL")
            .expect("DATABASE_TEST_URL must be set. Perhaps .env is missing?")
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
            &db_url(),
            PathBuf::from(test_file_storage_path),
        );

        assert!(fdb.is_ok(), "Failed to create database");

        fdb.unwrap()
    }

    lazy_static! {
        // Most functions that modify the database already want
        // `Arc<Mutex<fdb>>` so it has to have two layers of mutex
        static ref FDB: Arc<Mutex<FileDatabase>>
                = Arc::new(Mutex::new(create_db()));
    }

    pub fn get_database() -> Arc<Mutex<FileDatabase>> {
        FDB.clone()
    }

    pub fn run_test<F: Fn(&FileDatabase)>(test: F) {
        let fdb = FDB.lock().unwrap();
        fdb.reset();
        assert_eq!(fdb.get_file_amount(), 0);

        test(&fdb);
    }

    //////////////////////////////////////////////////
    // Helper functions
    //////////////////////////////////////////////////
    pub fn get_files_with_tags(fdb: &FileDatabase, tags: Vec<String>, negated: Vec<String>)
        -> Vec<File>
    {
        let file_query = search::SavedSearchQuery::with_tags((tags, negated));

        fdb.search_files(file_query)
    }

    pub fn get_file_paths_with_tags(fdb: &FileDatabase, tags: Vec<String>, negated: Vec<String>) 
        -> Vec<String>
    {
        get_files_with_tags(fdb, tags, negated)
            .iter()
            .map(|file| file.filename.clone())
            .collect()
    }


    pub fn get_file_paths_from_files(files: &[File]) -> Vec<String> {
        let mut result = vec![];

        for file in files {
            result.push(file.filename.clone());
        }

        result
    }
}

/*
   Tests
 */
#[cfg(test)]
mod db_tests {
    use super::*;

    use changelog::ChangeCreationPolicy;

    use std::sync::Arc;

    use chrono::{NaiveDate, Datelike};

    use date_search;

    use super::db_test_helpers::{
        get_files_with_tags,
        get_file_paths_with_tags,
        get_file_paths_from_files
    };

    //////////////////////////////////////////////////
    // Tests
    //////////////////////////////////////////////////

    db_test!{add_test(fdb) {
        fdb.add_new_file(
            1,
            "test1",
            Some("thumb1"),
            &vec!["tag1".to_string(), "tag2".to_string()],
            0,
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "test2",
            Some("thumb2"),
            &vec!["tag1".to_string(), "tag3".to_string()],
            0,
            ChangeCreationPolicy::No
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
    }}

    db_test!{multi_tag_test(fdb) {
        fdb.add_new_file(
            1,
            "test1",
            Some("thumb1"),
            &vec!["common_tag".to_string(), "only1_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "test2",
            Some("thumb2"),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            3,
            "test3",
            Some("thumb3"),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
            ChangeCreationPolicy::No
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
    }}

    db_test!{modify_tags_test(fdb) {
        let file = fdb.add_new_file(
            1,
            "test1",
            Some("thumb1"),
            &vec!["old_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );

        fdb.change_file_tags(&file, &vec!["new_tag".to_string()], ChangeCreationPolicy::No)
            .unwrap();

        assert!(
            get_files_with_tags(&fdb, vec!["new_tag".to_string()], vec![])
                .len() == 1
        );
        assert!(
            get_files_with_tags(&fdb, vec!["old_tag".to_string()], vec![])
                .len() == 0
        );
    }}


    db_test!{negated_tags_test(fdb) {
        fdb.add_new_file(
            1,
            "test1",
            Some("thumb1"),
            &vec!["common_tag".to_string(), "only1_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "test2",
            Some("thumb2"),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            3,
            "test3",
            Some("thumb3"),
            &vec!["common_tag".to_string(), "only2_3_tag".to_string()],
            0,
            ChangeCreationPolicy::No
        );

        let result = get_files_with_tags(
            &fdb,
            vec!["common_tag".to_string()],
            vec!["only1_tag".to_string()],
        );
        assert!(get_file_paths_from_files(&result).contains(&"test1".to_owned()) == false);
        assert!(get_file_paths_from_files(&result).contains(&"test2".to_owned()));
        assert!(get_file_paths_from_files(&result).contains(&"test3".to_owned()));
    }}

    db_test!{timestamp_search(fdb) {
        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &vec!["tag1".to_owned(), "tag2".to_owned()],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb2"),
                &vec!["tag1".to_owned(), "tag3".to_owned()],
                NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                3,
                "file3",
                Some("thumb2"),
                &vec!["tag1".to_owned(), "tag3".to_owned()],
                NaiveDate::from_ymd(2017,06,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        let intervals = vec!(
                date_search::Interval::new(
                    NaiveDate::from_ymd(2016,05,01).and_hms(0,0,0),
                    NaiveDate::from_ymd(2017,05,01).and_hms(0,0,0)
                )
            );

        let interval_result = fdb.search_files(
            search::SavedSearchQuery::with_date_constraints(
                date_search::DateConstraints::with_intervals(intervals)
            )
        );

        assert!(get_file_paths_from_files(&interval_result).contains(&"file1".to_owned()));
        assert!(get_file_paths_from_files(&interval_result).contains(&"file2".to_owned()) == false);
        assert!(get_file_paths_from_files(&interval_result).contains(&"file3".to_owned()) == false);

        let functions: Vec<Arc<date_search::DateConstraintFunction>> = vec!(
                Arc::new(|date: &NaiveDateTime| date.year() == 2017),
                Arc::new(|date: &NaiveDateTime| date.month0() == 0),
            );

        let function_result = fdb.search_files(
                search::SavedSearchQuery::with_date_constraints(
                    date_search::DateConstraints::with_constraints(functions)
                )
            );

        assert!(get_file_paths_from_files(&function_result).contains(&"file1".to_owned()));
        assert!(get_file_paths_from_files(&function_result).contains(&"file2".to_owned()) == false);
        assert!(get_file_paths_from_files(&function_result).contains(&"file3".to_owned()) == false);
    }}

    db_test!{empty_search_should_return_all_files(fdb) {
        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &vec!["tag1".to_owned(), "tag2".to_owned()],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb2"),
                &vec!["tag1".to_owned(), "tag3".to_owned()],
                NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                3,
                "file3",
                Some("thumb2"),
                &vec!["tag1".to_owned(), "tag3".to_owned()],
                NaiveDate::from_ymd(2017,06,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        let result = fdb.search_files(search::SavedSearchQuery::empty());

        assert_eq!(result.len(), 3);
    }}

    db_test!{files_should_be_ordered_by_date(fdb) {
        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &vec![],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb2"),
                &vec![],
                NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                3,
                "file3",
                Some("thumb2"),
                &vec![],
                NaiveDate::from_ymd(2017,06,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        let result = fdb.search_files(search::SavedSearchQuery::empty());

        let mut last_date = result[0].creation_date;
        for file in result {
            if file.creation_date < last_date {
                panic!("Files are not ordered by date. {:?} > {:?}", last_date, file.creation_date);
            }
            last_date = file.creation_date;
        }
    }}

    /**
      Investigates bug with changelog changes affecting multiple files
    */
    db_test!{update_only_updates_the_affected_file(fdb) {
        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &mapvec![String::from: "tag"],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb2"),
                &mapvec![String::from: "tag"],
                NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        let mut file = fdb.get_file_with_id(1).unwrap();
        assert_eq!(file.id, 1);
        file.tags = vec!();
        fdb.update_file_without_creating_change(&file).expect("File update failed");

        let files_with_tag = get_file_paths_with_tags(&fdb, vec!["tag".to_string()], vec![]);

        assert!( files_with_tag.contains(&"file1".to_string()) == false);
        assert!( files_with_tag .contains(&"file2".to_string()));
    }}

    db_test!{file_drop_works(fdb) {
        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &mapvec![String::from: "tag"],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb2"),
                &mapvec![String::from: "tag"],
                NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        fdb.drop_file(2, ChangeCreationPolicy::No).unwrap();

        let file = fdb.get_file_with_id(1);
        assert_matches!(file, Some(_));
        let file = fdb.get_file_with_id(2);
        assert_matches!(file, None);
    }}
}

#[cfg(test)]
mod chage_tests {
    use super::*;

    use changelog::ChangeCreationPolicy;

    use chrono::{NaiveDate};

    use changelog::{ChangeType, Change, UpdateType};


    /**
      Ensures that the correct changes get added to the database.
    */
    db_test!{file_additions_create_changes(fdb) {
        let first_file_timestamp = NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0);
        let second_file_timestamp = NaiveDate::from_ymd(2017,1,2).and_hms(0,0,0);

        fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &mapvec![String::from: "tag"],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::Yes(first_file_timestamp)
            );
        fdb.add_new_file(
                2,
                "file2",
                Some("thumb1"),
                &vec!(),
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::Yes(second_file_timestamp)
            );

        let changes = fdb.get_all_changes().unwrap();
        assert_eq!(changes.len(), 3); // Two additions, one tag addition

        assert_eq!(changes[0], Change::new(
            first_file_timestamp,
            1,
            ChangeType::FileAdded,
        ));
        assert_eq!(changes[1], Change::new(
            first_file_timestamp,
            1,
            ChangeType::Update(UpdateType::TagAdded("tag".into())),
        ));
        assert_eq!(changes[2], Change::new(
            second_file_timestamp,
            2,
            ChangeType::FileAdded,
        ));
    }}

    /**
      Ensures that tag removals and additions create changes
    */
    db_test!{tag_edits_create_changes(fdb) {
        let timestamp = NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0);

        // Set up a file with a single tag
        let file = fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &mapvec![String::from: "tag", "yolo"],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        // Set new tags for the file. Yolo is kept and no change should be created for it
        fdb.change_file_tags(
            &file,
            &mapvec![String::from: "yolo", "swag"],
            ChangeCreationPolicy::Yes(timestamp)
        ).expect("File tag change failed");

        let changes = fdb.get_all_changes().expect("Failed to get changes from database");
        assert_eq!(changes.len(), 2);


        assert!(changes.contains(&Change::new(
            timestamp,
            1,
            ChangeType::Update(UpdateType::TagAdded("swag".into())),
        )));
        assert!(changes.contains(&Change::new(
            timestamp,
            1,
            ChangeType::Update(UpdateType::TagRemoved("tag".into())),
        )));
    }}

    db_test!{timestamp_changes_create_edits(fdb) {
        let change_timestamp = NaiveDate::from_ymd(2017,10,1).and_hms(0,0,0);
        let new_timestamp = NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0);

        // Set up a file with a single tag
        let file = fdb.add_new_file(
                1,
                "file1",
                Some("thumb1"),
                &mapvec![String::from: "tag", "yolo"],
                NaiveDate::from_ymd(2017,01,01).and_hms(0,0,0).timestamp() as u64,
                ChangeCreationPolicy::No
            );

        // Set new tags for the file. Yolo is kept and no change should be created for it
        fdb.set_file_timestamp(
            &file,
            new_timestamp,
            ChangeCreationPolicy::Yes(change_timestamp)
        ).expect("Failed to set file timestamp");

        let changes = fdb.get_all_changes().expect("FAiled to get changes from database");
        assert_eq!(changes.len(), 1);


        assert!(changes.contains(&Change::new(
            change_timestamp,
            1,
            ChangeType::Update(UpdateType::CreationDateChanged(new_timestamp)),
        )));
    }}
}
