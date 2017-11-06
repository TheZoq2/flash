use chrono::NaiveDateTime;

use std::convert::From;
use serde_json;

use schema::{changes, syncpoints};

use file_util::get_file_extension;
use file_database;

use error::{Error, ErrorKind, Result};
use std::path::PathBuf;

/*
  Change synchronisation:

  User (userver) requests a sync with a remote server (rserver).

  userver asks for a list of syncpoints from rserver
  userver receives syncpoints and compares with its own.
  userver finds common syncpoint and requests all changes after
    that syncpoint
  userver sends this common syncpoint to rserver
    rserver starts own sync process
  userver requests all changes after common syncpoint
  userver requests additional data from rserver (filenames, file types)
  userver applies changes
  userver creates a new syncpoint and stores all changes in db
*/

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ChangeType {
    FileAdded,
    FileRemoved,
    TagAdded(String),
    TagRemoved(String),
    CreationDateChanged(NaiveDateTime)
}

#[derive(PartialEq, Debug, Clone)]
pub struct Change {
    pub change_type: ChangeType,
    pub affected_file: i32,
    timestamp: NaiveDateTime
}

impl Change {
    pub fn new(timestamp: NaiveDateTime, affected_file: i32, change_type: ChangeType) -> Change {
        Change {
            timestamp,
            affected_file,
            change_type
        }
    }
}

#[derive(Queryable)]
pub struct ChangeDbEntry {
    id: i32,
    timestamp: NaiveDateTime,
    json_data: String,
    affected_file: i32,
}

impl From<Change> for ChangeDbEntry {
    fn from(other: Change) -> Self {
        Self {
            id: 0,
            json_data: serde_json::to_string(&other.change_type).unwrap(),
            affected_file: other.affected_file,
            timestamp: other.timestamp
        }
    }
}

#[derive(Insertable)]
#[table_name="changes"]
pub struct InsertableChange<'a> {
    json_data: &'a str,
    affected_file: i32,
    timestamp: NaiveDateTime
}


#[derive(Queryable, PartialEq, Clone, Debug)]
pub struct SyncPoint {
    pub last_change: NaiveDateTime
}

#[derive(Insertable)]
#[table_name="syncpoints"]
pub struct InsertableSyncPoint {
    last_change: NaiveDateTime
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

#[cfg(test)]
mod syncpoint_tests {
    use super::*;

    fn timestamp_from_string(date_string: &str) -> Result<NaiveDateTime, ::chrono::ParseError> {
        NaiveDateTime::parse_from_str(
                     &format!("{} 00:00:00", date_string),
                     "%Y-%m-%d %H:%M:%S"
                )
    }

    fn syncpoint_from_string(date_string: &str) -> Result<SyncPoint, ::chrono::ParseError> {
        Ok(SyncPoint{
            last_change: timestamp_from_string(date_string)?
        })
    }

    #[test]
    fn finding_common_syncpoint_should_work() {
        let local = vec!(
                syncpoint_from_string("2015-09-05").unwrap(),
                syncpoint_from_string("2015-10-05").unwrap(),
                syncpoint_from_string("2016-10-05").unwrap(),
            );
        let remote = vec!(
                syncpoint_from_string("2015-09-05").unwrap(),
                syncpoint_from_string("2015-10-05").unwrap(),
                syncpoint_from_string("2016-06-05").unwrap(),
            );

        let last_common = last_common_syncpoint(&local, &remote);
        assert_eq!(last_common, Some(syncpoint_from_string("2015-10-05").unwrap()));
    }

    fn disjoint_syncpoints_should_not_have_common() {
        let local = vec!(
                syncpoint_from_string("2015-09-03").unwrap(),
                syncpoint_from_string("2015-10-04").unwrap(),
                syncpoint_from_string("2016-10-05").unwrap(),
            );
        let remote = vec!(
                syncpoint_from_string("2015-09-05").unwrap(),
                syncpoint_from_string("2015-10-05").unwrap(),
                syncpoint_from_string("2016-06-05").unwrap(),
            );

        let last_common = last_common_syncpoint(&local, &remote);
        assert_eq!(last_common, None);
    }

    #[test]
    fn changes_are_merged_properly() {
        let changeset1 = vec!(
                Change::new(timestamp_from_string("2016-09-05").unwrap(), 0, ChangeType::FileAdded),
                Change::new(timestamp_from_string("2016-09-05").unwrap(), 0, ChangeType::TagAdded(String::from("some_tag"))),
                Change::new(timestamp_from_string("2016-10-05").unwrap(), 1, ChangeType::TagRemoved(String::from("some_other_tag"))),
                Change::new(timestamp_from_string("2016-10-11").unwrap(), 1, ChangeType::FileRemoved)
            );
        let changeset2 = vec!(
                Change::new(timestamp_from_string("2016-09-06").unwrap(), 2, ChangeType::FileAdded),
                Change::new(timestamp_from_string("2016-09-06").unwrap(), 2, ChangeType::TagAdded(String::from("yolo"))),
                Change::new(timestamp_from_string("2016-10-05").unwrap(), 1, ChangeType::TagAdded(String::from("some_other_tag")))
            );

        let merged_changes = merge_changes(&changeset1, &changeset2);

        let mut last_timestamp = merged_changes[0].timestamp;
        for change in merged_changes {
            if change.timestamp < last_timestamp {
                panic!("changes are not ordered by date");
            }
            last_timestamp = change.timestamp
        }
    }
}
