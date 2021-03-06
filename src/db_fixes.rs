use file_database::FileDatabase;
use error::Result;

use chrono::NaiveDateTime;

use changelog::{ChangeType, Change, UpdateType};


#[allow(dead_code)]
pub fn create_changes_for_files(fdb: &FileDatabase, timestamp: &NaiveDateTime) -> Result<()> {
    let files = fdb.search_files(::search::SavedSearchQuery::empty());

    let already_added_files = fdb.get_all_changes().unwrap().iter()
        .filter_map(|change| {
            match change.change_type {
                ChangeType::FileAdded => Some(change.affected_file),
                _ => None
            }
        })
        .collect::<Vec<_>>();

    for file in files {
        if already_added_files.contains(&file.id) {
            continue
        }
        fdb.add_change(&Change::new(*timestamp, file.id, ChangeType::FileAdded))?;

        for tag in file.tags {
            fdb.add_change(&Change::new(
                *timestamp,
                file.id,
                ChangeType::Update(UpdateType::TagAdded(tag.to_string()))
            ))?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn deduplicate_tags(fdb: &FileDatabase) -> Result<()> {
    for mut file in fdb.search_files(::search::SavedSearchQuery::empty()) {
        let mut unique_tags = vec!();

        for tag in file.tags {
            if !unique_tags.contains(&tag) {
                unique_tags.push(tag)
            }
        }

        file.tags = unique_tags;
        fdb.update_file_without_creating_change(&file)?;
    }

    Ok(())
}

#[cfg(test)]
mod add_change_tests {
    use super::*;

    use changelog::ChangeCreationPolicy;
    use changelog::{ChangeType, Change, UpdateType};

    use chrono::NaiveDate;

    db_test!(file_changes_are_created(fdb) {
        // Add some files to the database
        fdb.add_new_file(
            1,
            "some_filename",
            None,
            &mapvec!(String::from: "image1", "shared"),
            100,
            &ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "some_other_filename",
            None,
            &mapvec!(String::from: "image2", "shared"),
            150,
            &ChangeCreationPolicy::No
        );

        // Ensure that no changes were created
        assert_eq!(fdb.get_all_changes().unwrap().len(), 0);

        // Add changes
        let timestamp = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        create_changes_for_files(fdb, &timestamp).unwrap();


        // Ensure that changes were created
        let expected_changes = ::changelog::sorted_changes(
            &vec!(
                Change::new(timestamp, 1, ChangeType::FileAdded),
                Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("image1".to_owned()))),
                Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("shared".to_owned()))),
                Change::new(timestamp, 2, ChangeType::FileAdded),
                Change::new(timestamp, 2, ChangeType::Update(UpdateType::TagAdded("image2".to_owned()))),
                Change::new(timestamp, 2, ChangeType::Update(UpdateType::TagAdded("shared".to_owned()))),
            )
        );

        let changes = fdb.get_all_changes().expect("Failed to get all changes");

        assert_eq!(changes, expected_changes);
    });

    db_test!(existing_file_changes_are_not_created(fdb) {
        // Add some files to the database
        fdb.add_new_file(
            1,
            "some_filename",
            None,
            &mapvec!(String::from: "image1", "shared"),
            100,
            &ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "some_other_filename",
            None,
            &mapvec!(String::from: "image2", "shared"),
            150,
            &ChangeCreationPolicy::No
        );

        // Add changes
        let timestamp = NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0);
        fdb.add_change(&Change::new(timestamp, 1, ChangeType::FileAdded)).unwrap();
        fdb.add_change(&Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("image1".to_owned())))).unwrap();
        fdb.add_change(&Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("shared".to_owned())))).unwrap();

        assert_eq!(fdb.get_all_changes().unwrap().len(), 3);

        create_changes_for_files(fdb, &timestamp).unwrap();


        // Ensure that changes were created
        let expected_changes = ::changelog::sorted_changes(
            &vec!(
                Change::new(timestamp, 1, ChangeType::FileAdded),
                Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("image1".to_owned()))),
                Change::new(timestamp, 1, ChangeType::Update(UpdateType::TagAdded("shared".to_owned()))),
                Change::new(timestamp, 2, ChangeType::FileAdded),
                Change::new(timestamp, 2, ChangeType::Update(UpdateType::TagAdded("image2".to_owned()))),
                Change::new(timestamp, 2, ChangeType::Update(UpdateType::TagAdded("shared".to_owned()))),
            )
        );

        let changes = fdb.get_all_changes().expect("Failed to get all changes");

        assert_eq!(changes, expected_changes);
    });

    db_test!(file_tag_deduplication_works(fdb) {
        fdb.add_new_file(
            1,
            "some_filename",
            None,
            &mapvec!(String::from: "image1", "shared", "image1", "image1"),
            100,
            &ChangeCreationPolicy::No
        );

        deduplicate_tags(fdb).unwrap();

        let file = fdb.get_file_with_id(1).unwrap();
        assert_eq!(file.tags, mapvec!(String::from: "image1", "shared"));
    });
}
