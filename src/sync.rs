use changelog::{Change, SyncPoint, ChangeType, UpdateType, ChangeCreationPolicy};

use byte_source::{ByteSource};

use file_database::{FileDatabase};
use error::{Result, ErrorKind, ResultExt};
use file_handler;
use file_handler::ThumbnailStrategy;

use chrono::prelude::*;

use foreign_server::{ForeignServer, FileDetails};



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
    // Get the syncpoints from the local and remote servers
    let local_syncpoints = fdb.get_syncpoints()
        .chain_err(|| "Failed to get local syncpoints")?;
    let remote_syncpoints = foreign_server.get_syncpoints()
        .chain_err(|| "Failed to get remote syncpoints")?;

    // Find the highest common syncpoint
    let sync_merge_start = last_common_syncpoint(&local_syncpoints, &remote_syncpoints);

    // Get the changes that have been made locally since that change
    let local_changes = match sync_merge_start {
        Some(ref syncpoint) => fdb.get_changes_after_timestamp(&syncpoint.last_change),
        None => fdb.get_all_changes()
    }.chain_err(|| "Failed to get local changes")?;

    // Find all files that have been removed localy
    let local_removed_files: Vec<_> = local_changes.iter()
        .filter_map(|change| match change.change_type {
            ChangeType::FileRemoved => Some(change.affected_file),
            _ => None
        })
        .collect();

    // Fetch all remote changes that have been made on the remote server
    let remote_changes = foreign_server.get_changes(&sync_merge_start)
        .chain_err(|| "Failed to get remote changes")?;

    // Create a new syncpoint
    let new_syncpoint = SyncPoint{
            last_change: NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0)
        };

    // Send the changes to the remote server to apply
    foreign_server.send_changes(&local_changes, &new_syncpoint)
        .chain_err(|| "Failed to send changes")?;

    // Apply changes locally
    apply_changes(fdb, foreign_server, &remote_changes, &local_removed_files)
        .chain_err(|| "Failed to apply changes")
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
                let file_details = foreign_server.get_file_details(change.affected_file)
                    .chain_err(|| "Failed to get fille details")?;

                let file = ByteSource::Memory(
                    foreign_server.get_file(change.affected_file)
                        .chain_err(|| "Failed to get file")?
                );
                let thumbnail = {
                    let from_server = foreign_server.get_thumbnail(change.affected_file)
                        .chain_err(|| "Failed to get thumbnail")?;
                    match from_server {
                        Some(data) =>
                            ThumbnailStrategy::FromByteSource(ByteSource::Memory(data)),
                        None => ThumbnailStrategy::None
                    }
                };

                let file_timestamp = file_details.timestamp;

                file_handler::save_file(
                            file,
                            thumbnail,
                            change.affected_file,
                            &[],
                            fdb,
                            ChangeCreationPolicy::No,
                            &file_details.extension,
                            file_timestamp.timestamp() as u64
                        ).chain_err(|| "Failed to save file")?;
            }
            ChangeType::FileRemoved => {
                file_handler::remove_file(change.affected_file, fdb, ChangeCreationPolicy::No)?;
            }
        }
    }

    for change in changes {
        fdb.add_change(change)?;
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
        UpdateType::CreationDateChanged(date) => file.creation_date = date
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

    use chrono;

    fn create_change(date_string: &str) -> chrono::format::ParseResult<ChangeCreationPolicy> {
        Ok(ChangeCreationPolicy::Yes(naive_datetime_from_date(date_string)?))
    }

    struct MockForeignServer {
        file_data: HashMap<i32, (FileDetails, Vec<u8>, Option<Vec<u8>>)>,
        syncpoints: Vec<SyncPoint>,
        changes: Vec<Change>
    }

    impl MockForeignServer {
        pub fn new(
            files: Vec<(i32, (FileDetails, Vec<u8>, Option<Vec<u8>>))>,
            syncpoints: Vec<SyncPoint>,
            changes: Vec<Change>
        ) -> Self {
            let mut file_data = HashMap::new();
            for (id, details) in files {
                file_data.insert(id, details);
            }
            Self {
                file_data,
                syncpoints,
                changes
            }
        }
    }

    impl ForeignServer for MockForeignServer {
        fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>{
            Ok(self.syncpoints.clone())
        }
        fn get_changes(&self, starting_syncpoint: &Option<SyncPoint>) -> Result<Vec<Change>> {
            match *starting_syncpoint {
                Some(SyncPoint{last_change}) => {
                    Ok(self.changes.iter()
                        .filter(|change| change.timestamp >= last_change)
                        .map(|change| change.clone())
                        .collect()
                    )
                },
                None => Ok(self.changes.iter().map(|change| change.clone()).collect())
            }
        }
        fn get_file_details(&self, id: i32) -> Result<FileDetails> {
            Ok(self.file_data[&id].0.clone())
        }

        fn send_changes(&self, _: &[Change], _: &SyncPoint) -> Result<()> {
            Ok(())
        }
        fn get_file(&self, id: i32) -> Result<Vec<u8>> {
            Ok(self.file_data[&id].1.clone())
        }
        fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
            Ok(self.file_data[&id].2.clone())
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
        fdb.add_new_file(1, "yolo.jpg", None, &vec!(), 0, create_change("2017-02-02").unwrap());
        fdb.add_new_file(2, "swag.jpg", None, &vec!(), 0, create_change("2017-02-02").unwrap());

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagAdded("things".into()))
                ),
            );

        apply_changes(fdb, &MockForeignServer::new(vec!(), vec!(), vec!()), &changes, &vec!()).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(1))
    }

    fn only_tag_removals(fdb: &mut FileDatabase) {
        fdb.add_new_file(1, "yolo.jpg", None, &mapvec!(String::from: "things"), 0, create_change("2017-02-02").unwrap());
        fdb.add_new_file(2, "swag.jpg", None, &mapvec!(String::from: "things"), 0, create_change("2017-02-02").unwrap());

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagRemoved("things".into()))
                ),
            );

        apply_changes(
            fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!()
        ).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    fn tag_removals_and_additions(fdb: &mut FileDatabase) {
        fdb.add_new_file(
            1,
            "yolo.jpg",
            None,
            &mapvec!(String::from: "things"),
            0,
            create_change("2017-02-02").unwrap()
        );
        fdb.add_new_file(
            2,
            "swag.jpg",
            None,
            &vec!(),
            0,
            create_change("2017-02-02").unwrap()
        );

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

        apply_changes(
            fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!()
        ).unwrap();

        let files_with_tag = get_files_with_tags(fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    fn creation_date_updates(fdb: &mut FileDatabase) {
        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        let new_timestamp = naive_datetime_from_date("2017-01-02").unwrap();
        fdb.add_new_file(1,
                         "yolo.jpg",
                         Some("t_yolo.jpg"),
                         &mapvec!(String::from: "things"),
                         original_timestamp.timestamp() as u64,
                         create_change("2017-02-02").unwrap()
                    );


        let changes = vec!(
                Change::new(
                    new_timestamp,
                    1,
                    ChangeType::Update(UpdateType::CreationDateChanged(new_timestamp))
                ),
            );

        apply_changes(
            fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!()
        ).unwrap();

        let file = fdb.get_file_with_id(1).unwrap();

        assert_eq!(file.creation_date, new_timestamp);
    }

    fn file_system_changes_work(fdb: &mut FileDatabase) {
        // Set up the intial database
        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        let original_filename = "test/media/10x10.png";

        let (initial_file, worker_receiver) = file_handler::save_file(
            ByteSource::File(PathBuf::from(original_filename)),
            ThumbnailStrategy::Generate,
            1,
            &mapvec!(String::from: "things"),
            fdb,
            create_change("2017-02-02").unwrap(),
            "jpg",
            original_timestamp.timestamp() as u64
        ).expect("Failed to save initial file");

        //Wait for the file to be saved and ensure that no error was thrown
        let save_result = worker_receiver.file.recv()
            .expect("File saving worker did not send result");
        save_result.expect("File saving failed");
        // Wait for the thumbnail to be generatad and ensure that no error was thrown
        let thumbnail_result = worker_receiver
            .thumbnail
            .expect("Thumbnail generator channel not created")
            .recv()
            .expect("Thumbnail worker did not send result");
        thumbnail_result.expect("Thumbnail generation failed");

        // Set up the foreign server
        let added_bytes = include_bytes!("../test/media/DSC_0001.JPG").into_iter()
            .map(|a| *a)
            .collect::<Vec<_>>();
        let added_thumbnail_bytes = include_bytes!("../test/media/512x512.png").into_iter()
            .map(|a| *a)
            .collect::<Vec<_>>();


        // Set up foreign changes
        let remote_changes = vec!(
                Change::new(
                    original_timestamp,
                    3,
                    ChangeType::FileAdded
                ),
                Change::new(
                    original_timestamp,
                    2,
                    ChangeType::FileAdded
                ),
                Change::new(
                    original_timestamp,
                    1,
                    ChangeType::FileRemoved
                ),
            );

        let mut all_changes = vec!();
        for change in &remote_changes {
            all_changes.push(change.clone());
        }
        for change in &fdb.get_all_changes().expect("Failed to get changes from db") {
            all_changes.push(change.clone());
        }
        all_changes.sort_by_key(|change| change.timestamp);

        // Set up the foreign server
        let foreign_server = MockForeignServer::new(
                vec!(
                    (2, (FileDetails::new(
                        "jpg".into(),
                        NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
                    ), added_bytes, Some(added_thumbnail_bytes))),
                    (3, (FileDetails::new(
                        "jpg".into(),
                        NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
                    ), vec!(), None)),
                ),
                vec!(),
                remote_changes
            );

        // Apply the changes
        sync_with_foreign(fdb, &foreign_server).expect("Foreign server sync failed");

        // Assert that the local database now contains all changes
        assert_eq!(
            all_changes,
            fdb.get_all_changes().expect("Failed to get changes from database")
        );


        // Ensure that the correct files are in the database
        let file_1 = fdb.get_file_with_id(1);
        let file_2 = fdb.get_file_with_id(2);
        let file_3 = fdb.get_file_with_id(3);
        assert_eq!(file_1, None);
        assert_matches!(file_2, Some(_));
        assert_matches!(file_3, Some(_));

        let (file_2, file_3) = (
            file_2.expect("File 2 was added"),
            file_3.expect("File 3 was added")
        );

        // Open the file and compare the bytes to the expected values
        let actual_file_2 = PathBuf::from(file_2.filename);
        let actual_thumbnail_2 = file_2.thumbnail_path.map(|tp| PathBuf::from(tp));

        let actual_file_3 = PathBuf::from(file_3.filename);
        assert_matches!(file_3.thumbnail_path, None);

        // Ensure that the old file was deleted
        {
            let path = fdb.get_file_save_path().join(PathBuf::from(initial_file.filename));
            assert!(!path.exists());
        }

        // Ensure that the old thumbnail was deleted
        {
            let path = fdb.get_file_save_path()
                .join(PathBuf::from(
                    initial_file.thumbnail_path.expect("Initial file had no thumbnail")
                ));
            assert!(!path.exists());
        }

        // Ensure that the new are created
        {
            let path = fdb.get_file_save_path().join(actual_file_2);
            assert!(path.exists());
        }
        {
            let path = fdb.get_file_save_path().join(actual_file_3);
            assert!(path.exists());
        }

        // Ensure that the new thumbnail for file 2 was created
        {
            let path = fdb.get_file_save_path()
                .join(PathBuf::from(
                    actual_thumbnail_2.expect("File 2 should have a filename")
                ));
            assert!(path.exists());
        }
        // Ensure that the no file for file 3 was created
        {
            let path = fdb.get_file_save_path()
                .join(PathBuf::from(
                    "thumb_3.jpg"
                ));
            assert!(!path.exists());
        }
    }
}
