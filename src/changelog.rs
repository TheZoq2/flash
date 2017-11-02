use chrono::NaiveDateTime;

use std::convert::From;
use serde_json;

use schema::{changes, syncpoints};

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
    change_type: String,
    affected_file: i32,
    timestamp: NaiveDateTime
}

impl From<Change> for ChangeDbEntry {
    fn from(other: Change) -> Self {
        Self {
            change_type: serde_json::to_string(&other.change_type).unwrap(),
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

pub fn merge_changes(changeset1: &[Change], changeset2: &[Change]) -> Vec<Change> {
    let mut merged = changeset1.iter()
        .chain(changeset2.iter())
        .map(|x| (*x).clone())
        .collect::<Vec<_>>();

    merged.sort_by_key(|x| x.timestamp);

    return merged
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
