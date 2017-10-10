use chrono::NaiveDateTime;

use std::convert::From;
use serde_json;

use schema::changes;

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
