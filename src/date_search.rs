use chrono::NaiveDateTime;

use std::vec::Vec;
use std::str::{FromStr, SplitWhitespace};

enum TimeParseError {
    UnexpectedWord(String),
    UnexpectedEndOfQuery
}

pub struct Interval {
    start: NaiveDateTime,
    end: NaiveDateTime
}

enum TimeDescriptor {
    Day,
    Week,
    Month,
    Year
}

enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December
}

impl Month {
    pub fn as_index(&self) -> u32 {
        match *self {
            Month::January => 0,
            Month::February => 1,
            Month::March => 2,
            Month::April => 3,
            Month::May => 4,
            Month::June => 5,
            Month::July => 6,
            Month::August => 7,
            Month::September => 8,
            Month::October => 9,
            Month::November => 10,
            Month::December => 11,
        }
    }
}

impl FromStr for Month {
    type Err = TimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "january" => Ok(Month::January),
            "february" => Ok(Month::February),
            "march" => Ok(Month::March),
            "april" => Ok(Month::April),
            "may" => Ok(Month::May),
            "june" => Ok(Month::June),
            "july" => Ok(Month::July),
            "august" => Ok(Month::August),
            "september" => Ok(Month::September),
            "october" => Ok(Month::October),
            "november" => Ok(Month::November),
            "december" => Ok(Month::December),
            other => Err(TimeParseError::UnexpectedWord(other.to_owned()))
        }
    }
}

pub type DateConstraintFunction = Fn(NaiveDateTime) -> bool;

pub fn parse_date_query(query: &str, current_time: &NaiveDateTime)
    -> (Vec<Interval>, Vec<Box<DateConstraintFunction>>)
{
    unimplemented!()
}

fn tokenise_time_descriptor(query: &mut SplitWhitespace, current_time: &NaiveDateTime)
    -> Result<TimeDescriptor, TimeParseError>
{
    match query.next() {
        Some(word) => {
            match word {
                "day" => Ok(TimeDescriptor::Day),
                "week" => Ok(TimeDescriptor::Week),
                "month" => Ok(TimeDescriptor::Month),
                "year" => Ok(TimeDescriptor::Year),
                other => Err(TimeParseError::UnexpectedWord(other.to_owned()))
            }
        },
        None => Err(TimeParseError::UnexpectedEndOfQuery)
    }
}
