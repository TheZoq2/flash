use changelog::{Change, SyncPoint, ChangeType, UpdateType, ChangeCreationPolicy};

use file_database::{FileDatabase};
use error::{Result, ErrorKind};
use file_handler;

use chrono::prelude::*;

pub struct FileDetails {
    pub extension: String,
    pub file: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>
}

impl FileDetails {
    pub fn new(extension: String, file: Vec<u8>, thumbnail: Option<Vec<u8>>) -> Self {
        Self { extension, file, thumbnail }
    }
}

pub trait ForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>;
    fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>>;
    fn get_file_details(&self, id: i32) -> Result<FileDetails>;
    fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()>;
    fn get_file(&self, id: i32) -> Result<Vec<u8>>;
    fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>>;
}



pub fn last_common_syncpoint(local: &[SyncPoint], remote: &[SyncPoint])
    -> Option<SyncPoint>
{
    local.iter().zip(remote.iter())
        .fold(None, |acc, (l, r)| {
            if l == r {
                Some(l.clone())
            }
            else {
                acc
            }
        })
}

pub fn sync_with_foreign(fdb: &FileDatabase, foreign_server: &ForeignServer) -> Result<()> {
    let local_syncpoints = fdb.get_syncpoints()?;
    let remote_syncpoints = foreign_server.get_syncpoints()?;

    let sync_merge_start = last_common_syncpoint(&local_syncpoints, &remote_syncpoints);

    let local_changes = match sync_merge_start {
        Some(ref syncpoint) => fdb.get_changes_after_timestamp(&syncpoint.last_change)?,
        None => fdb.get_all_changes()?
    };
    let local_removed_files: Vec<_> = local_changes.iter()
        .filter_map(|change| match change.change_type {
            ChangeType::FileRemoved => Some(change.affected_file),
            _ => None
        })
        .collect();

    let remote_changes = foreign_server.get_changes(&sync_merge_start)?;

    let new_syncpoint = SyncPoint{
            last_change: NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0)
        };

    foreign_server.send_changes(&local_changes, &new_syncpoint)?;

    apply_changes(fdb, foreign_server, &remote_changes, &local_removed_files)
}


fn apply_changes(
        fdb: &FileDatabase,
        foreign_server: &ForeignServer,
        changes: &[Change],
        local_removed_files: &[i32],
    ) -> Result<()>
{
    let changes_to_be_applied = changes.iter().filter(|change| {
        !local_removed_files.contains(&change.affected_file)
    });

    for change in changes_to_be_applied {
        match change.change_type {
            ChangeType::Update(ref update_type) => {
                apply_file_update(fdb, change.affected_file, update_type)?
            }
            ChangeType::FileAdded => {
                let file_details = foreign_server.get_file_details(change.affected_file)?;

                let file_timestamp = unimplemented!("Fetch timestamp from foreign server");

                file_handler::save_file(
                            //This needs to create a bytesource by downloading it from the foreign
                            //server
                            &file_details.file,
                            &file_details.thumbnail,
                            change.affected_file,
                            &vec!(),
                            fdb,
                            ChangeCreationPolicy::No,
                            &file_details.extension,
                            file_timestamp
                        );
            }
            ChangeType::FileRemoved => {
                file_handler::remove_file(change.affected_file, fdb, ChangeCreationPolicy::No)?;
            }
        }
    }

    for change in changes {
        fdb.add_change(change);
    }

    Ok(())
}


fn apply_file_update(fdb: &FileDatabase, affected_file: i32, file_update: &UpdateType)
    -> Result<()>
{
    let mut file = match fdb.get_file_with_id(affected_file) {
        Some(id) => id,
        None => return Err(ErrorKind::NoSuchFileInDatabase(affected_file).into())
    };

    match *file_update {
        UpdateType::TagAdded(ref tag) => file.tags.push(tag.clone()),
        UpdateType::TagRemoved(ref tag) => {
            file.tags = file.tags.into_iter().filter(|t| t != tag).collect()
        }
        UpdateType::CreationDateChanged(date) => file.creation_date = Some(date)
    }

    fdb.update_file_without_creating_change(&file)?;
    Ok(())
}


#[cfg(test)]
mod sync_tests {
    use super::*;

    use std::collections::HashMap;

    use file_database::db_test_helpers;

    use test_macros::naive_datetime_from_date;

    use file_database::db_test_helpers::get_files_with_tags;

    use std::path::PathBuf;

    struct MockForeignServer {
        file_data: HashMap<i32, FileDetails>
    }

    impl MockForeignServer {
        pub fn new(files: Vec<(i32, FileDetails)>) -> Self {
            let mut file_data = HashMap::new();
            for (id, details) in files {
                file_data.insert(id, details);
            }
            Self {
                file_data
            }
        }
    }

    impl ForeignServer for MockForeignServer {
        fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>{
            unimplemented!()
        }
        fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>> {
            unimplemented!()
        }
        fn get_file_details(&self, id: i32) -> Result<FileDetails> {
            unimplemented!()
        }
        fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()> {
            unimplemented!()
        }
        fn get_file(&self, id: i32) -> Result<Vec<u8>> {
            unimplemented!()
        }
        fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
            unimplemented!()
        }
    }

    #[test]
    fn db_tests() {
        db_test_helpers::run_test(only_tag_additions);
        db_test_helpers::run_test(only_tag_removals);
        db_test_helpers::run_test(tag_removals_and_additions);
        db_test_helpers::run_test(creation_date_updates);
        db_test_helpers::run_test(file_system_changes_work);
    }

    fn only_tag_additions(fdb: &mut FileDatabase) {
        fdb.add_new_file(1, "yolo.jpg", None, &vec!(), 0);
        fdb.add_new_file(2, "swag.jpg", None, &vec!(), 0);

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagAdded("things".into()))
                ),
            );

        apply_changes(fdb, &MockForeignServer::new(vec!()), &changes, &vec!()).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(1))
    }

    fn only_tag_removals(fdb: &mut FileDatabase) {
        fdb.add_new_file(1, "yolo.jpg", None, &mapvec!(String::from: "things"), 0);
        fdb.add_new_file(2, "swag.jpg", None, &mapvec!(String::from: "things"), 0);

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagRemoved("things".into()))
                ),
            );

        apply_changes(fdb, &MockForeignServer::new(vec!()), &changes, &vec!()).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    fn tag_removals_and_additions(fdb: &mut FileDatabase) {
        fdb.add_new_file(1, "yolo.jpg", None, &mapvec!(String::from: "things"), 0);
        fdb.add_new_file(2, "swag.jpg", None, &vec!(), 0);

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagRemoved("things".into()))
                ),
                Change::new(
                    naive_datetime_from_date("2017-01-02").unwrap(),
                    2,
                    ChangeType::Update(UpdateType::TagAdded("things".into()))
                ),
                Change::new(
                    naive_datetime_from_date("2017-01-02").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagRemoved("things".into()))
                ),
            );

        apply_changes(fdb, &MockForeignServer::new(vec!()), &changes, &vec!()).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    fn creation_date_updates(fdb: &mut FileDatabase) {
        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        let new_timestamp = naive_datetime_from_date("2017-01-02").unwrap();
        fdb.add_new_file(1, "yolo.jpg", Some("t_yolo.jpg"), &mapvec!(String::from: "things")
                         , original_timestamp.timestamp() as u64);


        let changes = vec!(
                Change::new(
                    new_timestamp,
                    1,
                    ChangeType::Update(UpdateType::CreationDateChanged(new_timestamp))
                ),
            );

        apply_changes(fdb, &MockForeignServer::new(vec!()), &changes, &vec!()).unwrap();

        let file = fdb.get_file_with_id(1).unwrap();

        assert_eq!(file.creation_date, Some(new_timestamp));
    }

    fn file_system_changes_work(fdb: &mut FileDatabase) {
        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();

        let original_filename = "yolo.jpg";
        fdb.add_new_file(1, original_filename, None, &mapvec!(String::from: "things")
                         , original_timestamp.timestamp() as u64);

        let added_bytes = include_bytes!("../test/media/DSC_0001.JPG").into_iter()
            .map(|a| *a)
            .collect::<Vec<_>>();
        let added_thumbnail_bytes = include_bytes!("../test/media/512x512.png").into_iter()
            .map(|a| *a)
            .collect::<Vec<_>>();

        let foreign_server = MockForeignServer::new(
                vec!(
                    (2, FileDetails::new(
                            "jpg".into(),
                            added_bytes.clone(),
                            Some(added_thumbnail_bytes)
                        )
                    ),
                    (3, FileDetails::new("jpg".into(), added_bytes.clone(), None))
                )
            );

        let changes = vec!(
                Change::new(
                    original_timestamp,
                    2,
                    ChangeType::FileAdded
                ),
                Change::new(
                    original_timestamp,
                    3,
                    ChangeType::FileAdded
                ),
                Change::new(
                    original_timestamp,
                    1,
                    ChangeType::FileRemoved
                ),
            );

        apply_changes(fdb, &foreign_server, &changes, &vec!()).unwrap();

        // Ensure that the correct files are in the database
        let file_1 = fdb.get_file_with_id(1);
        let file_2 = fdb.get_file_with_id(2);
        let file_3 = fdb.get_file_with_id(3);
        assert_eq!(file_1, None);
        assert_matches!(file_2, Some(_));
        assert_matches!(file_3, Some(_));

        let (file_2, file_3) = (file_2.unwrap(), file_3.unwrap());

        // Open the file and compare the bytes to the expected values
        let actual_file_2 = PathBuf::from(file_2.filename);
        let actual_thumbnail_2 = file_2.thumbnail_path.map(|tp| PathBuf::from(tp));

        let actual_file_3 = PathBuf::from(file_3.filename);
        assert_matches!(file_3.thumbnail_path, None);

        unimplemented!("Make sure new files are created");
        // Ensure that the old file was deleted
        {
            let path = fdb.get_file_save_path().join(PathBuf::new(original_filename));
            assert!(!path.exists())
        }
    }
}
