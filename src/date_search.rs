use chrono::NaiveDateTime;

use std::vec::Vec;

pub struct Interval {
    start: NaiveDateTime,
    end: NaiveDateTime
}

pub type DateConstraintFunction = Fn(NaiveDateTime) -> bool;

pub fn parse_date_query(query: &str) -> (Vec<Interval>, Vec<Box<DateConstraintFunction>>) {
    unimplemented!()
}
