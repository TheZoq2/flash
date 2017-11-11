use changelog::{Change, SyncPoint, ChangeType, UpdateType};

use file_database::{FileDatabase};
use error::{Result, ErrorKind};

use chrono::prelude::*;

struct FileDetails {
    extension: String
}

trait ForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>;
    fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>>;
    fn get_file_details(&self, id: i32) -> Result<FileDetails>;
    fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()>;
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
                unimplemented!()
            }
            ChangeType::FileRemoved => {
                unimplemented!()
            }
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
    }

    #[test]
    fn db_tests() {
        db_test_helpers::run_test(only_tag_additions);
        db_test_helpers::run_test(only_tag_removals);
    }

    fn only_tag_additions(fdb: &mut FileDatabase) {
        fdb.add_new_file(1, "yolo.jpg", "t_yolo.jpg", &vec!(), 0);
        fdb.add_new_file(2, "swag.jpg", "t_swag.jpg", &vec!(), 0);

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
        fdb.add_new_file(1, "yolo.jpg", "t_yolo.jpg", &mapvec!(String::from: "things"), 0);
        fdb.add_new_file(2, "swag.jpg", "t_swag.jpg", &mapvec!(String::from: "things"), 0);

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
}