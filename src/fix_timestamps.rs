use file_database::FileDatabase;

use search::SavedSearchQuery;

use file_util;

fn fix_timestamps(fdb: &FileDatabase) {
    // Fetch all files
    let files = fdb.search_files(SavedSearchQuery::empty());

    // Read their creation time from exif. If it fails do nothing. Else update the file
    for file in files {
        let path = fdb.get_file_save_path().join(&file.filename);

        match file_util::get_file_timestamp_from_metadata(&path) {
            Ok(Some(actual_creation_date)) => fdb.set_file_timestamp(&file, &actual_creation_date).unwrap(),
            Ok(None) => {},
            Err(e) => {
                println!("Failed to read file timestamp: {}", e);
            }
        }
    }
}


#[cfg(test)]
mod timestamp_fix_tests {
    use super::*;

    use file_database::db_test_helpers;

    use std::fs;

    use chrono::{NaiveDate, NaiveDateTime};

    #[test]
    fn timestamp_fix_test_runner() {
        db_test_helpers::run_test(timestamp_fix_test);
    }

    fn timestamp_fix_test(fdb: &mut FileDatabase) {
        // Copy test-files to storage location
        fs::copy("test/media/DSC_0001.JPG", fdb.get_file_save_path().join("DSC_0001.JPG")).unwrap();
        fs::copy("test/media/512x512.png", fdb.get_file_save_path().join("512x512.png")).unwrap();

        let first_file_id = fdb.add_new_file("DSC_0001.JPG", "yolo", &vec!(), 500).id;
        let second_file_id = fdb.add_new_file("512x512.png", "yolo", &vec!(), 500).id;

        fix_timestamps(fdb);

        let first_file = fdb.get_file_with_id(first_file_id).unwrap();
        let second_file = fdb.get_file_with_id(second_file_id).unwrap();
        assert_eq!(first_file.creation_date, Some(NaiveDate::from_ymd(2016,12,16).and_hms(21,34,26)));
        assert_eq!(second_file.creation_date, Some(NaiveDateTime::from_timestamp(500, 0)));
    }
}
