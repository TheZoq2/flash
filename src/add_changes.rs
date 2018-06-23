use file_database::FileDatabase;
use error::Result;

use chrono::NaiveDateTime;

use changelog::{ChangeType, Change, UpdateType};


pub fn create_changes_for_files(fdb: &FileDatabase, timestamp: &NaiveDateTime) -> Result<()> {
    let files = fdb.search_files(::search::SavedSearchQuery::empty());

    for file in files {
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
            ChangeCreationPolicy::No
        );
        fdb.add_new_file(
            2,
            "some_other_filename",
            None,
            &mapvec!(String::from: "image2", "shared"),
            150,
            ChangeCreationPolicy::No
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

        let changes =fdb.get_all_changes().expect("Failed to get all changes");

        assert_eq!(changes, expected_changes);
    });
}
