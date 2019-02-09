use chrono::NaiveDateTime;

use std::hash::{Hasher, Hash};
use std::collections::hash_map::DefaultHasher;
use std::convert::From;
use serde_json;

use schema::{changes, syncpoints};

use error::Result;

use std::cmp::Ordering;


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash)]
pub enum UpdateType {
    TagAdded(String),
    TagRemoved(String),
    CreationDateChanged(NaiveDateTime)
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash)]
pub enum ChangeType {
    FileAdded,
    FileRemoved,
    Update(UpdateType)
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash)]
pub struct Change {
    pub id: i32,
    pub change_type: ChangeType,
    pub affected_file: i32,
    pub timestamp: NaiveDateTime
}

impl Change {
    pub fn new(timestamp: NaiveDateTime, affected_file: i32, change_type: ChangeType) -> Change {
        let mut hasher = DefaultHasher::default();
        timestamp.hash(&mut hasher);
        affected_file.hash(&mut hasher);
        change_type.hash(&mut hasher);
        let id = hasher.finish() as i32;
        Change {
            id: id,
            timestamp,
            affected_file,
            change_type
        }
    }

    pub fn from_db_entry(db_entry: &ChangeDbEntry) -> Result<Self> {
        Ok(Self {
            id: db_entry.id,
            affected_file: db_entry.affected_file,
            timestamp: db_entry.timestamp,
            change_type: serde_json::from_str(&db_entry.json_data)?
        })
    }
}

fn type_order_int(change_type: &ChangeType) -> u8 {
    match *change_type {
        ChangeType::FileAdded => 0,
        ChangeType::Update(_) => 1,
        ChangeType::FileRemoved => 2
    }
}
/**
  Sorts a given vector of changes first according to timestamps and if there
  are conflicts it ensures that additions happen before updates which happens
  before removals
*/
pub fn sorted_changes(changes: &[Change]) -> Vec<Change> {

    let mut result = changes.to_vec();

    result.sort_by(|change1, change2| {
        match change1.timestamp.cmp(&change2.timestamp) {
            Ordering::Equal => {
                type_order_int(&change1.change_type).cmp(&type_order_int(&change2.change_type))
            }
            not_equal => not_equal
        }
    });


    result
}

#[derive(Queryable)]
pub struct ChangeDbEntry {
    id: i32,
    timestamp: NaiveDateTime,
    json_data: String,
    affected_file: i32,
}

impl<'a> From<&'a Change> for ChangeDbEntry {
    fn from(other: &Change) -> Self {
        Self {
            id: other.id,
            json_data: serde_json::to_string(&other.change_type).unwrap(),
            affected_file: other.affected_file,
            timestamp: other.timestamp
        }
    }
}

#[derive(Insertable)]
#[table_name="changes"]
pub struct InsertableChange<'a> {
    id: i32,
    json_data: &'a str,
    affected_file: i32,
    timestamp: NaiveDateTime
}

impl<'a> From<&'a ChangeDbEntry> for InsertableChange<'a> {
    fn from(other: &'a ChangeDbEntry) -> Self {
        Self {
            id: other.id,
            json_data: &other.json_data,
            affected_file: other.affected_file,
            timestamp: other.timestamp
        }
    }
}


#[derive(Queryable, Insertable, Serialize, Deserialize, PartialEq, Clone, Debug, PartialOrd, Ord, Eq)]
#[table_name="syncpoints"]
pub struct SyncPoint {
    pub last_change: NaiveDateTime
}

#[derive(Clone)]
pub enum ChangeCreationPolicy {
    Yes(NaiveDateTime),
    No
}



#[cfg(test)]
mod changelog_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn change_sorting() {
        let change_20160401 = Change::new(
            NaiveDate::from_ymd(2016,04,01).and_hms(0,0,0),
            0,
            ChangeType::FileAdded,
        );
        let change_20160101 = Change::new(
            NaiveDate::from_ymd(2016,01,01).and_hms(0,0,0),
            0,
            ChangeType::FileAdded,
        );
        let change_20160301 = Change::new(
            NaiveDate::from_ymd(2016,03,01).and_hms(0,0,0),
            0,
            ChangeType::FileRemoved,
        );
        let change_20160201 = Change::new(
            NaiveDate::from_ymd(2016,02,01).and_hms(0,0,0),
            0,
            ChangeType::FileAdded,
        );
        let changes = vec!(
            change_20160401.clone(),
            change_20160101.clone(),
            change_20160301.clone(),
            change_20160301.clone(),
            change_20160201.clone(),
            change_20160201.clone(),
        );

        let expected_order = vec!(
            change_20160101.clone(),
            change_20160201.clone(),
            change_20160201.clone(),
            change_20160301.clone(),
            change_20160301.clone(),
            change_20160401.clone(),
        );

        assert_eq!(sorted_changes(&changes), expected_order);
    }
}
