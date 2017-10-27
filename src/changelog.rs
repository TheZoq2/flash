use chrono::NaiveDateTime;

use std::convert::From;
use serde_json;

use schema::{changes, syncpoints};

#[derive(Serialize, Deserialize)]
pub enum ChangeType {
    FileAdded(i32),
    FileRemoved(i32),
    TagAdded(i32, String),
    TagRemoved(i32, String),
    CreationDateChanged(i32, NaiveDateTime)
}

pub struct Change {
    change_type: ChangeType,
    timestamp: NaiveDateTime
}

impl Change {
    pub fn new(timestamp: NaiveDateTime, change_type: ChangeType) -> Change {
        Change {
            timestamp,
            change_type
        }
    }
}

#[derive(Queryable)]
pub struct ChangeDbEntry {
    change_type: String,
    timestamp: NaiveDateTime
}

impl From<Change> for ChangeDbEntry {
    fn from(other: Change) -> Self {
        Self {
            change_type: serde_json::to_string(&other.change_type).unwrap(),
            timestamp: other.timestamp
        }
    }
}

#[derive(Insertable)]
#[table_name="changes"]
pub struct InsertableChange<'a> {
    json_data: &'a str,
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

    fn syncpoint_from_string(date_string: &str) -> Result<SyncPoint, ::chrono::ParseError> {
        Ok(SyncPoint{
            last_change: NaiveDateTime::parse_from_str(
                             &format!("{} 00:00:00", date_string),
                             "%Y-%m-%d %H:%M:%S"
                        )?
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

    fn function_name() {
        
    }
}
