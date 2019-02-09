use changelog::{
    Change,
    SyncPoint,
    ChangeType,
    UpdateType,
    ChangeCreationPolicy,
};

use byte_source::{ByteSource};

use file_database::{FileDatabase};
use error::{Result, ErrorKind, ResultExt};
use file_handler;
use file_handler::{remove_file, ThumbnailStrategy};
use foreign_server::{ForeignServer, ChangeData};
use sync_progress as sp;

use chrono::prelude::*;

use std::thread;



pub fn last_common_syncpoint(local: &[SyncPoint], remote: &[SyncPoint])
    -> Option<SyncPoint>
{
    let mut local = local.iter().collect::<Vec<_>>();
    local.sort();
    let mut remote = remote.iter().collect::<Vec<_>>();
    remote.sort();
    let mut last_common = None;
    for (r, l) in local.iter().zip(remote.iter()) {
        if r == l {
            last_common = Some(r.clone());
        }
        else {
            break;
        }
    }
    last_common.cloned()
}

fn get_removed_files(changes: &[Change]) -> Vec<i32> {
    changes.iter()
        .filter_map(|change| match change.change_type {
            ChangeType::FileRemoved => Some(change.affected_file),
            _ => None
        })
        .collect()
}

pub fn sync_with_foreign(
    fdb: &FileDatabase,
    foreign_server: &mut ForeignServer,
    own_port: u16,
    progress_reporter: &sp::LocalTxType
) -> Result<()> {
    let (job_id, progress_tx) = progress_reporter;

    let (
        local_changes,
        new_local_syncpoints,
        new_remote_syncpoints,
        removed_files,
        remote_changes
    ) = {
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
        // Fetch all remote changes that have been made on the remote server
        let remote_changes = foreign_server.get_changes(&sync_merge_start)
            .chain_err(|| "Failed to get remote changes")?;

        // Find all files that have been removed
        let mut removed_files = vec!();
        removed_files.extend_from_slice(&get_removed_files(&local_changes));
        removed_files.extend_from_slice(&get_removed_files(&remote_changes));

        let mut new_local_syncpoints = remote_syncpoints.clone().into_iter()
                    .filter(|p| !local_syncpoints.contains(p))
                    .collect::<Vec<_>>();
        let mut new_remote_syncpoints = local_syncpoints.clone().into_iter()
                    .filter(|p| !remote_syncpoints.contains(p))
                    .collect::<Vec<_>>();

        // Create a new syncpoint
        let new_syncpoint = SyncPoint{
                last_change: NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0)
            };

        new_local_syncpoints.push(new_syncpoint.clone());
        new_remote_syncpoints.push(new_syncpoint.clone());

        progress_tx.send((*job_id, sp::SyncUpdate::GatheredData))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

        (local_changes, new_local_syncpoints, new_remote_syncpoints, removed_files, remote_changes)
    };

    // Send the changes to the remote server to apply
    let foreign_job_id = foreign_server.send_changes(
        &ChangeData{
            changes: local_changes,
            removed_files: removed_files.clone()
        },
        own_port
    )
        .chain_err(|| "Failed to send changes")?;

    progress_tx.send((*job_id, sp::SyncUpdate::SentToForeign(foreign_job_id)))
        .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

    // Apply changes locally
    apply_changes(fdb, foreign_server, &remote_changes, &removed_files, progress_reporter)
        .chain_err(|| "Failed to apply changes")?;

    progress_tx.send((*job_id, sp::SyncUpdate::AddingSyncpoint))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

    // Wait for the foreign server to finnish its sync
    progress_tx.send((*job_id, sp::SyncUpdate::WaitingForForeign))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

    // We will retry this a couple of times because it is possible
    // for things to happen before the other side adds the job to its
    // list
    let mut error_amount = 0;
    loop {
        let status = foreign_server.get_sync_status(foreign_job_id);
        match status {
            Ok(status) => if let sp::SyncUpdate::Done = status.last_update {
                break;
            },
            Err(e) => {
                if error_amount > 5 {
                    return Err(e)
                }
                else {
                    error_amount += 1
                }
            }
        }

        thread::sleep(::std::time::Duration::from_secs(1));
    }

    for point in new_remote_syncpoints {
        foreign_server.add_syncpoint(&point)
            .chain_err(|| "Failed to apply syncpoint on foreign")?;
    }

    progress_tx.send((*job_id, sp::SyncUpdate::AddingSyncpoint))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

    for point in new_local_syncpoints {
        fdb.add_syncpoint(&point)?;
    }

    progress_tx.send((*job_id, sp::SyncUpdate::Done))
        .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

    Ok(())
}


/**
  Applies the specified changes to the database. Any changes affecting files in 
  the `removed_files` vec are ignored and the files are removed

  The function does not check for changes that are already in the database which
  means that such changes would be duplicated.
*/
pub fn apply_changes(
        fdb: &FileDatabase,
        foreign_server: &ForeignServer,
        changes: &[Change],
        removed_files: &[i32],
        (job_id, progress_tx): &sp::LocalTxType,
    ) -> Result<()>
{
    let changes_to_be_applied = changes.iter().filter(|change| {
        !removed_files.contains(&change.affected_file)
    }).collect::<Vec<_>>();

    let mut changes_left = changes_to_be_applied.len();
    for change in changes_to_be_applied {
        changes_left -= 1;
        progress_tx.send((
            *job_id,
            sp::SyncUpdate::StartingToApplyChange(changes_left)
        ))
        .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

        apply_change(fdb, change, foreign_server)
            .chain_err(|| {
                format!(
                    "Failed to apply change, affected file: {}",
                    change.affected_file
                )
            })?;
    }

    let mut changes_to_be_added = changes.len();
    for change in changes {
        changes_to_be_added -= 1;
        progress_tx.send((*job_id, sp::SyncUpdate::AddingChangeToDb(changes_to_be_added)))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

        fdb.add_change(change)?;
    }

    let mut files_to_remove = removed_files.len();
    for id in removed_files {
        files_to_remove -= 1;
        progress_tx.send((*job_id, sp::SyncUpdate::RemovingFile(files_to_remove)))
            .unwrap_or_else(|_e| println!("Warning: Sync progress listener crashed"));

        if fdb.get_file_with_id(*id) != None {
            remove_file(*id, &fdb, &ChangeCreationPolicy::No)?;
        }
    }

    Ok(())
}

fn fetch_with_retries(server: &ForeignServer, file_id: i32, max_retries: usize) -> Result<Vec<u8>> {
    let mut last_err = None;
    for _ in 0..max_retries+1 {
        let file_bytes = server.get_file(file_id);
        match file_bytes {
            Ok(bytes) => {return Ok(bytes)}
            Err(e) => {
                println!("Warning: failed to fetch file, retrying");
                println!("{}", e);
                last_err = Some(e)
            }
        }
    }
    return Err(last_err.unwrap())
}

fn apply_change(
    fdb: &FileDatabase,
    change: &Change,
    foreign_server: &ForeignServer
) -> Result<()> {
    match change.change_type {
        ChangeType::Update(ref update_type) => {
            apply_file_update(&fdb, change.affected_file, update_type)?
        }
        ChangeType::FileAdded => {
            // Check if the file is already in the database if it is, ignore it and print
            // a warning
            if fdb.get_file_with_id(change.affected_file) == None {
                let file_details = foreign_server.get_file_details(change.affected_file)
                    .chain_err(|| "Failed to get fille details")?;

                let file = ByteSource::Memory(
                    fetch_with_retries(foreign_server, change.affected_file, 1)?
                );

                let thumbnail = {
                    let from_server = foreign_server.get_thumbnail(change.affected_file)
                        .unwrap_or_else(|e| {
                            println!("Failed to get thumbnail, defaulting to None. Error: {:?}", e);
                            None
                        });

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
                            &fdb,
                            &ChangeCreationPolicy::No,
                            &file_details.extension,
                            file_timestamp.timestamp() as u64
                        ).chain_err(|| "Failed to save file")?;
            }
            else {
                println!(
                    "A file with id {} was already in the database. Ignoring",
                    change.affected_file
                );
            }
        }
        ChangeType::FileRemoved => {
            file_handler::remove_file(change.affected_file, &fdb, &ChangeCreationPolicy::No)?;
        }
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
        UpdateType::TagAdded(ref tag) => {
            if !file.tags.contains(&tag) {
                file.tags.push(tag.clone())
            }
        },
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

    use sync_progress::{SyncStatus, SyncUpdate};

    use foreign_server::{FileDetails};

    use chrono;

    use std::sync::Mutex;

    fn create_change(date_string: &str) -> chrono::format::ParseResult<ChangeCreationPolicy> {
        Ok(ChangeCreationPolicy::Yes(naive_datetime_from_date(date_string)?))
    }

    struct MockForeignServer {
        file_data: HashMap<i32, (FileDetails, Vec<u8>, Option<Vec<u8>>)>,
        // This is a mutex to allow modification without the compiler getting
        // pissed because add_syncpoint isn't mut
        syncpoints: Mutex<Vec<SyncPoint>>,
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
                syncpoints: Mutex::new(syncpoints),
                changes
            }
        }
    }

    impl ForeignServer for MockForeignServer {
        fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>{
            Ok(self.syncpoints.lock().unwrap().clone())
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

        fn send_changes(&mut self, data: &ChangeData, _port: u16) -> Result<usize> {
            let mut changes = data.changes.clone();
            self.changes.append(&mut changes);
            Ok(0)
        }
        fn get_file(&self, id: i32) -> Result<Vec<u8>> {
            Ok(self.file_data[&id].1.clone())
        }
        fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
            Ok(self.file_data[&id].2.clone())
        }
        fn get_sync_status(&self, _job_id: usize) -> Result<SyncStatus> {
            Ok(SyncStatus{last_update: SyncUpdate::Done, foreign_job_id: None})
        }
        fn add_syncpoint(&self, syncpoint: &SyncPoint) -> Result<()> {
            self.syncpoints.lock().unwrap().push(syncpoint.clone());
            Ok(())
        }
    }

    #[test]
    fn only_tag_additions() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        fdb.add_new_file(1, "yolo.jpg", None, &vec!(), 0, &create_change("2017-02-02").unwrap());
        fdb.add_new_file(2, "swag.jpg", None, &vec!(), 0, &create_change("2017-02-02").unwrap());

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagAdded("things".into()))
                ),
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).unwrap();

        let files_with_tag = get_files_with_tags(&fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(1))
    }

    #[test]
    fn only_tag_removals() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        fdb.add_new_file(1, "yolo.jpg", None, &mapvec!(String::from: "things"), 0, &create_change("2017-02-02").unwrap());
        fdb.add_new_file(2, "swag.jpg", None, &mapvec!(String::from: "things"), 0, &create_change("2017-02-02").unwrap());

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagRemoved("things".into()))
                ),
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).unwrap();

        let files_with_tag = get_files_with_tags(&fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    #[test]
    fn tag_removals_and_additions() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        fdb.add_new_file(
            1,
            "yolo.jpg",
            None,
            &mapvec!(String::from: "things"),
            0,
            &create_change("2017-02-02").unwrap()
        );
        fdb.add_new_file(
            2,
            "swag.jpg",
            None,
            &vec!(),
            0,
            &create_change("2017-02-02").unwrap()
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

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).unwrap();

        let files_with_tag = get_files_with_tags(&fdb, mapvec!(String::from: "things"), vec!());

        let matched_ids: Vec<_> = files_with_tag.iter()
            .map(|file| file.id)
            .collect();

        assert_eq!(matched_ids, vec!(2))
    }

    #[test]
    fn creation_date_updates() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        let new_timestamp = naive_datetime_from_date("2017-01-02").unwrap();
        fdb.add_new_file(1,
                         "yolo.jpg",
                         Some("t_yolo.jpg"),
                         &mapvec!(String::from: "things"),
                         original_timestamp.timestamp() as u64,
                         &create_change("2017-02-02").unwrap()
                    );


        let changes = vec!(
                Change::new(
                    new_timestamp,
                    1,
                    ChangeType::Update(UpdateType::CreationDateChanged(new_timestamp))
                ),
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).unwrap();

        let file = fdb.get_file_with_id(1).unwrap();

        assert_eq!(file.creation_date, new_timestamp);
    }

    #[test]
    fn file_already_in_database_does_not_abort() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        fdb.add_new_file(1,
                         "yolo.jpg",
                         Some("t_yolo.jpg"),
                         &mapvec!(String::from: "things"),
                         timestamp.timestamp() as u64,
                         &create_change("2017-02-02").unwrap()
                    );


        let changes = vec!(
                Change::new(
                    timestamp,
                    1,
                    ChangeType::FileAdded
                ),
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        assert_matches!(apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ), Ok(_));
    }

    #[test]
    fn duplicate_tag_changes_do_not_duplicate_tag() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        fdb.add_new_file(
            1,
            "yolo.jpg",
            None,
            &vec!("yolo".to_string(), "swag".to_string()),
            0,
            &create_change("2017-02-02").unwrap()
        );

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagAdded("yolo".into()))
                ),
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::Update(UpdateType::TagAdded("yolo".into()))
                ),
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        apply_changes(
            &fdb,
            &MockForeignServer::new(vec!(), vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).unwrap();

        let file = fdb.get_file_with_id(1).unwrap();
        assert_eq!(file.tags, vec!("yolo".to_string(), "swag".to_string()));
    }

    #[test]
    fn file_system_changes_work() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        // Set up the intial database
        let original_timestamp = naive_datetime_from_date("2017-01-01").unwrap();
        let original_filename = "test/media/10x10.png";

        let (initial_file, worker_receiver) = file_handler::save_file(
            ByteSource::File(PathBuf::from(original_filename)),
            ThumbnailStrategy::Generate,
            1,
            &mapvec!(String::from: "things"),
            &fdb,
            &create_change("2017-02-02").unwrap(),
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
        let mut foreign_server = MockForeignServer::new(
                vec!(
                    (2, (FileDetails {
                        extension: "jpg".into(),
                        timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
                    }, added_bytes, Some(added_thumbnail_bytes))),
                    (3, (FileDetails {
                        extension: "jpg".into(),
                        timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
                    }, vec!(), None)),
                ),
                vec!(),
                remote_changes
            );


        // Set up progress monitoring
        let (tx, _rx, _) = sp::setup_progress_datastructures();

        // Apply the changes
        sync_with_foreign(&fdb, &mut foreign_server, 0, &(0, tx.clone()))
            .expect("Foreign server sync failed");

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

        // Ensure that syncpoints were created
        let syncpoints = fdb
            .get_syncpoints()
            .expect("failed to read syncpoints from database");
        assert_eq!(syncpoints.len(), 1);

        let foreign_syncpoints = foreign_server.get_syncpoints();
        assert_eq!(foreign_syncpoints.expect("Failed to get syncpoints from foreign").len(), 1);
    }

    #[test]
    fn last_common_syncpoint_works() {
        let side1 = vec!(
            SyncPoint{last_change: naive_datetime_from_date("2018-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-02").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-03").unwrap()},
        );
        let side2 = vec!(
            SyncPoint{last_change: naive_datetime_from_date("2018-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-02").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-02-03").unwrap()},
        );

        assert_eq!(
            last_common_syncpoint(&side1, &side2).unwrap(),
            SyncPoint{last_change: naive_datetime_from_date("2019-01-02").unwrap()}
        );
    }

    #[test]
    fn only_changes_after_last_common_syncpoint_are_applied() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        let common_syncpoint =
            SyncPoint{last_change: NaiveDate::from_ymd(2017, 1, 1).and_hms(1,1,1)};

        fdb.reset();
        fdb.add_syncpoint(
            &common_syncpoint
        ).unwrap();
        fdb.add_new_file(
            1,
            "",
            None,
            &vec!(),
            0,
            &create_change("2017-02-02").unwrap()
        );
        fdb.add_change(
            &Change::new(
                NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0),
                1,
                ChangeType::FileAdded
            )
        ).expect("Failed to add change to databse");

        let foreign_files = vec!(
            (2, (FileDetails {
                extension: "jpg".into(),
                timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
            }, vec!(), None)),
            (3, (FileDetails {
                extension: "jpg".into(),
                timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
            }, vec!(), None)),
        );
        let foreign_syncpoints = vec!(common_syncpoint);
        let foreign_changes = vec!(
            Change::new(
                NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0),
                2,
                ChangeType::FileAdded
            ),
            Change::new(
                NaiveDate::from_ymd(2018, 1, 1).and_hms(0,0,0),
                3,
                ChangeType::FileAdded
            )
        );

        let mut server = MockForeignServer::new(
                foreign_files,
                foreign_syncpoints,
                foreign_changes
            );

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        sync_with_foreign(&fdb, &mut server, 0, &(0, tx)).unwrap();

        assert!(fdb.get_file_with_id(2).is_none());
        assert!(fdb.get_file_with_id(3).is_some());
        assert_eq!(server.changes.len(), 3);
    }

    #[test]
    fn last_common_syncpoint_considers_gaps() {
        let side1 = vec!(
            SyncPoint{last_change: naive_datetime_from_date("2017-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2018-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2018-02-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-02").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-03").unwrap()},
        );
        let side2 = vec!(
            SyncPoint{last_change: naive_datetime_from_date("2017-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2018-01-01").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-01-02").unwrap()},
            SyncPoint{last_change: naive_datetime_from_date("2019-02-03").unwrap()},
        );

        assert_eq!(
            last_common_syncpoint(&side1, &side2).unwrap(),
            SyncPoint{last_change: naive_datetime_from_date("2018-01-01").unwrap()}
        );
    }

    #[test]
    fn all_syncpoints_are_synced() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let common_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2016,1,1).and_hms(0,0,0)
        };
        let local_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 1, 1).and_hms(0,0,0)
        };
        let remote_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 2, 1).and_hms(0,0,0)
        };
        fdb.add_syncpoint(&common_syncpoint).expect("failed to add syncpoint");
        fdb.add_syncpoint(&local_only_syncpoint).expect("failed to add syncpoint");

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        let mut server = MockForeignServer::new(
                vec!(),
                vec!(common_syncpoint.clone(), remote_only_syncpoint.clone()),
                vec!()
            );
        sync_with_foreign(
            &fdb,
            &mut server,
            0,
            &(0, tx)
        ).expect("Failed to sync with foreign");

        let expected_syncpoints = vec!(
            common_syncpoint,
            local_only_syncpoint,
            remote_only_syncpoint
        );

        // Since we are adding a new syncpoint at the end, we expect it to be there
        // as well but we can't compare with it
        let mut local = fdb.get_syncpoints().expect("Failed to get syncpoints");
        local.pop();
        local.sort();
        assert_eq!(
            local,
            expected_syncpoints
        );

        let mut remote = server.syncpoints.lock().unwrap();
        remote.pop();
        remote.sort();
        assert_eq!(*remote, expected_syncpoints);
    }

    #[test]
    fn all_syncpoints_are_synced_if_none_are_common() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let local_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 1, 1).and_hms(0,0,0)
        };
        let remote_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 2, 1).and_hms(0,0,0)
        };
        fdb.add_syncpoint(&local_only_syncpoint).expect("failed to add syncpoint");

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        let mut server = MockForeignServer::new(
                vec!(),
                vec!(remote_only_syncpoint.clone()),
                vec!()
            );
        sync_with_foreign(
            &fdb,
            &mut server,
            0,
            &(0, tx)
        ).expect("Failed to sync with foreign");

        let expected_syncpoints = vec!(
            local_only_syncpoint,
            remote_only_syncpoint
        );

        // Since we are adding a new syncpoint at the end, we expect it to be there
        // as well but we can't compare with it
        let mut local = fdb.get_syncpoints().expect("Failed to get syncpoints");
        local.pop();
        local.sort();
        assert_eq!(
            local,
            expected_syncpoints
        );

        let mut remote = server.syncpoints.lock().unwrap();
        remote.pop();
        remote.sort();
        assert_eq!(*remote, expected_syncpoints);
    }

    #[test]
    fn common_syncpoint_detection_works_if_unsorted() {
        let sp1 = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 1, 1).and_hms(0,0,0)
        };
        let sp2 = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 2, 1).and_hms(0,0,0)
        };
        let sp3 = SyncPoint{
            last_change: NaiveDate::from_ymd(2018, 2, 1).and_hms(0,0,0)
        };

        let local = vec!(sp1.clone(), sp3.clone(), sp2.clone());
        let remote = vec!(sp3.clone(), sp2, sp1);

        let common = last_common_syncpoint(&local, &remote);

        assert_eq!(common, Some(sp3));
    }

    #[test]
    fn intermediate_duplicated_syncpoints_are_not_duplicated() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let common_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2016,1,1).and_hms(0,0,0)
        };
        let local_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 1, 1).and_hms(0,0,0)
        };
        let remote_only_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2017, 2, 1).and_hms(0,0,0)
        };
        let second_common_syncpoint = SyncPoint{
            last_change: NaiveDate::from_ymd(2018,1,1).and_hms(0,0,0)
        };
        fdb.add_syncpoint(&common_syncpoint).expect("failed to add syncpoint");
        fdb.add_syncpoint(&local_only_syncpoint).expect("failed to add syncpoint");
        fdb.add_syncpoint(&second_common_syncpoint).expect("failed to add syncpoint");

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        let mut server = MockForeignServer::new(
                vec!(),
                vec!(
                    common_syncpoint.clone(),
                    remote_only_syncpoint.clone(),
                    second_common_syncpoint.clone()
                ),
                vec!()
            );
        sync_with_foreign(
            &fdb,
            &mut server,
            0,
            &(0, tx)
        ).expect("Failed to sync with foreign");

        let expected_syncpoints = vec!(
            common_syncpoint,
            local_only_syncpoint,
            remote_only_syncpoint,
            second_common_syncpoint
        );

        // Since we are adding a new syncpoint at the end, we expect it to be there
        // as well but we can't compare with it
        let mut local = fdb.get_syncpoints().expect("Failed to get syncpoints");
        local.pop();
        local.sort();
        assert_eq!(
            local,
            expected_syncpoints
        );

        let mut remote = server.syncpoints.lock().unwrap();
        remote.pop();
        remote.sort();
        assert_eq!(*remote, expected_syncpoints);
    }



    struct ForeignServerWithThumbnailError {
        file_data: HashMap<i32, (FileDetails, Vec<u8>)>,
        syncpoints: Vec<SyncPoint>,
        changes: Vec<Change>,
    }

    impl ForeignServerWithThumbnailError {
        pub fn new(
            files: Vec<(i32, (FileDetails, Vec<u8>))>,
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
                changes,
            }
        }
    }

    impl ForeignServer for ForeignServerWithThumbnailError {
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

        fn send_changes(&mut self, _data: &ChangeData, _port: u16) -> Result<usize> {
            Ok(0)
        }
        fn get_file(&self, id: i32) -> Result<Vec<u8>> {
            Ok(self.file_data[&id].1.clone())
        }
        fn get_thumbnail(&self, _id: i32) -> Result<Option<Vec<u8>>> {
            Err(ErrorKind::Dummy.into())
        }
        fn get_sync_status(&self, _job_id: usize) -> Result<SyncStatus> {
            Ok(SyncStatus{last_update: SyncUpdate::Done, foreign_job_id: None})
        }
        fn add_syncpoint(&self, _syncpoint: &SyncPoint) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn applying_changes_with_thumbnail_error_does_not_crash() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let (tx, _rx, _) = sp::setup_progress_datastructures();

        let remote_changes = vec!(
                Change::new(
                    NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0),
                    2,
                    ChangeType::FileAdded
                ),
            );

        let foreign_server = ForeignServerWithThumbnailError::new(
                vec!(
                    (2, (FileDetails {
                        extension: "jpg".into(),
                        timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
                    }, vec!(0))),
                ),
                vec!(),
                remote_changes
            );

        apply_changes(
            &fdb,
            &foreign_server,
            &vec!(),
            &vec!(),
            &(0, tx)
        ).expect("Expected sync to work despite missing thumbnail");
    }

    struct UnstableForeignServer {
        file_data: HashMap<i32, (FileDetails, Vec<u8>, Mutex<bool>)>,
        syncpoints: Vec<SyncPoint>,
        changes: Vec<Change>,
    }

    impl UnstableForeignServer {
        pub fn new(
            files: Vec<(i32, (FileDetails, Vec<u8>))>,
            syncpoints: Vec<SyncPoint>,
            changes: Vec<Change>
        ) -> Self {
            let mut file_data = HashMap::new();
            for (id, (details, bytes)) in files {
                file_data.insert(id, (details, bytes, Mutex::new(false)));
            }
            Self {
                file_data,
                syncpoints,
                changes,
            }
        }
    }

    impl ForeignServer for UnstableForeignServer {
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

        fn send_changes(&mut self, _data: &ChangeData, _port: u16) -> Result<usize> {
            Ok(0)
        }
        fn get_file(&self, id: i32) -> Result<Vec<u8>> {
            let mut has_errored = self.file_data[&id].2.lock().unwrap();
            if *has_errored == true {
                Ok(self.file_data[&id].1.clone())
            }
            else {
                *has_errored = true;
                Err(ErrorKind::Dummy.into())
            }
        }
        fn get_thumbnail(&self, _id: i32) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }
        fn get_sync_status(&self, _job_id: usize) -> Result<SyncStatus> {
            Ok(SyncStatus{last_update: SyncUpdate::Done, foreign_job_id: None})
        }
        fn add_syncpoint(&self, _syncpoint: &SyncPoint) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn apply_changes_retries_failed_file_fetches() {
        let fdb = db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();
        fdb.reset();

        let changes = vec!(
                Change::new(
                    naive_datetime_from_date("2017-01-01").unwrap(),
                    1,
                    ChangeType::FileAdded
                ),
            );

        let foreign_files = vec!((
            1,
            (FileDetails {
                extension: "jpg".into(),
                timestamp: NaiveDate::from_ymd(2016, 1, 1).and_hms(0,0,0)
            }, vec!(1,2,3))
        ));

        let (tx, _rx, _) = sp::setup_progress_datastructures();
        assert!(apply_changes(
            &fdb,
            &UnstableForeignServer::new(foreign_files, vec!(), vec!()),
            &changes,
            &vec!(),
            &(0, tx)
        ).is_ok());
    }
}
