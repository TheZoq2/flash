use chrono::NaiveDateTime;

use std::convert::From;
use serde_json;

use schema::{changes, syncpoints};

use error::Result;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum UpdateType {
    TagAdded(String),
    TagRemoved(String),
    CreationDateChanged(NaiveDateTime)
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ChangeType {
    FileAdded,
    FileRemoved,
    Update(UpdateType)
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

    pub fn from_db_entry(db_entry: &ChangeDbEntry) -> Result<Self> {
        Ok(Self {
            affected_file: db_entry.affected_file,
            timestamp: db_entry.timestamp,
            change_type: serde_json::from_str(&db_entry.json_data)?
        })
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


#[derive(Queryable, Insertable, PartialEq, Clone, Debug)]
#[table_name="syncpoints"]
pub struct SyncPoint {
    pub last_change: NaiveDateTime
}
