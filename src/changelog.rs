use chrono::NaiveDateTime;

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
