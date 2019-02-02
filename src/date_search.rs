use chrono::{NaiveDateTime, NaiveTime, NaiveDate, Datelike, Duration, Weekday};

use std::vec::Vec;
use std::str::{FromStr, SplitWhitespace};

use std::sync::Arc;

#[derive(Debug)]
pub enum TimeParseError {
    UnexpectedWord(String),
    UnexpectedEndOfQuery
}

#[derive(Debug)]
#[derive(Clone)]
pub struct Interval {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime
}

impl Interval {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Interval {
        Interval {
            start,
            end
        }
    }

    pub fn contains(&self, time: &NaiveDateTime) -> bool {
        *time >= self.start && *time < self.end
    }
}

enum TimeDescriptor {
    Day,
    Week,
    Month,
    Year
}

impl FromStr for TimeDescriptor {
    type Err = TimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "day" => Ok(TimeDescriptor::Day),
            "week" => Ok(TimeDescriptor::Week),
            "month" => Ok(TimeDescriptor::Month),
            "year" => Ok(TimeDescriptor::Year),
            other => Err(TimeParseError::UnexpectedWord(other.to_owned()))
        }
    }
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
    fn as_number0(&self) -> u32 {
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
            Month::December => 11
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

pub type DateConstraintFunction = Fn(&NaiveDateTime) -> bool;

#[derive(Clone)]
pub struct DateConstraints {
    pub intervals: Vec<Interval>,
    // The function pointer is in an Arc because clone isn't implemented for
    // DateConstraintFunction
    pub constraints: Vec<Arc<DateConstraintFunction>>
}

impl DateConstraints {
    pub fn empty() -> Self {
        Self {
            intervals: vec!(),
            constraints: vec!()
        }
    }
    pub fn with_intervals(intervals: Vec<Interval>) -> Self {
        Self {
            intervals,
            constraints: vec!()
        }
    }

    pub fn with_constraints(constraints: Vec<Arc<DateConstraintFunction>>) -> Self {
        Self {
            intervals: vec!(),
            constraints
        }
    }

    /**
      Combines the current constraints with other constraint to create a new constraint that
      satisfies both originals
    */
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            intervals: self.intervals.iter()
                    .chain(other.intervals.iter())
                    .cloned()
                    .collect(),
            constraints: self.constraints.iter()
                    .chain(other.constraints.iter())
                    .cloned()
                    .collect(),
        }
    }
}

impl ::std::fmt::Debug for DateConstraints {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f,
               "DateConstraints {{ intervals: {:?}, {} constraint_functions }}",
               self.intervals,
               self.constraints.len()
            )
    }
}

pub fn parse_date_query(query: &str, current_time: &NaiveDateTime)
    -> Result<DateConstraints, TimeParseError>
{
    let mut words = query.split_whitespace();

    match words.next() {
        Some("this") =>
            Ok(DateConstraints::with_intervals(parse_modulu_search(&mut words, current_time)?)),
        Some("past") =>
            Ok(DateConstraints::with_intervals(parse_absolute_search(&mut words, current_time)?)),
        Some("in") | Some("on") =>
            Ok(DateConstraints::with_constraints(parse_date_pattern_search(&mut words)?)),
        // Special keywords, or unexpected tokens
        Some("since") => Ok(DateConstraints::with_intervals(parse_full_date_string(&mut words, current_time)?)),
        // Some("between") => unimplemented!(), TODO
        Some(other) => Err(TimeParseError::UnexpectedWord(other.to_string())),
        None => Err(TimeParseError::UnexpectedEndOfQuery)
    }
}


fn parse_modulu_search(query: &mut SplitWhitespace, current_time: &NaiveDateTime)
    -> Result<Vec<Interval>, TimeParseError>
{
    let time_descriptor = match query.next() {
        Some(word) => TimeDescriptor::from_str(word)?,
        None => return Err(TimeParseError::UnexpectedEndOfQuery)
    };

    let start_date = match time_descriptor {
        TimeDescriptor::Day => current_time.date(),
        TimeDescriptor::Week => {
            let week = current_time.iso_week();
            NaiveDate::from_isoywd(current_time.year(), week.week(), Weekday::Mon)
        }
        TimeDescriptor::Month => current_time.date().with_day0(0).unwrap(),
        TimeDescriptor::Year => current_time.date().with_day0(0)
                .unwrap()
                .with_month0(0).
                unwrap()
    };

    let start = NaiveDateTime::new(start_date, NaiveTime::from_hms_milli(0,0,0,0));

    Ok(vec!(Interval::new(start, *current_time)))
}

fn parse_absolute_search(query: &mut SplitWhitespace, current_time: &NaiveDateTime) 
    -> Result<Vec<Interval>, TimeParseError>
{
    let time_descriptor = match query.next() {
        Some(word) => TimeDescriptor::from_str(word)?,
        None => return Err(TimeParseError::UnexpectedEndOfQuery)
    };

    let subtracted_duration = match time_descriptor {
        TimeDescriptor::Day => Duration::days(1),
        TimeDescriptor::Week => Duration::weeks(1),
        TimeDescriptor::Month => Duration::days(30),
        TimeDescriptor::Year => Duration::days(365)
    };

    Ok(vec!(Interval::new(*current_time - subtracted_duration, *current_time)))
}


fn parse_date_pattern_search(query: &mut SplitWhitespace)
    -> Result<Vec<Arc<DateConstraintFunction>>, TimeParseError>
{
    let mut result_functions: Vec<Arc<DateConstraintFunction>>= vec!();

    for word in query {
        if let Ok(month) = Month::from_str(word) {
            result_functions.push(
                    Arc::new(move |date| date.month0() == month.as_number0())
                )
        }
        else if let Ok(number) = word.parse::<u32>() {
            if number < 31 {
                result_functions.push(
                        Arc::new(move |date| date.day() == number)
                    )
            }
            else {
                result_functions.push(
                        Arc::new(move |date| date.year() == number as i32)
                    )
            }
        }
    }

    Ok(result_functions)
}

fn parse_full_date_string(query: &mut SplitWhitespace, current_time: &NaiveDateTime)
    -> Result<Vec<Interval>, TimeParseError> 
{
    let datestring = match query.next() {
        Some(string) => string,
        None => return Err(TimeParseError::UnexpectedEndOfQuery)
    };

    match NaiveDate::parse_from_str(&datestring, "%Y-%m-%d") {
        Ok(date) => Ok(vec!(Interval::new(date.and_hms(0,0,0), *current_time))),
        Err(e) => {
            println!("{}", e);
            Err(TimeParseError::UnexpectedWord(datestring.to_string()))
        }
    }
}


#[cfg(test)]
mod parse_tests {
    use super::*;

    impl ::std::convert::From<TimeParseError> for String {
        fn from(error: TimeParseError) -> Self {
            format!("{:?}", error)
        }
    }

    fn timestamp_in_query_result(
            timestamp: &NaiveDateTime,
            &DateConstraints{ref intervals, ref constraints, ..}: &DateConstraints
        ) -> bool
    {
        for interval in intervals {
            if !interval.contains(timestamp) {
                return false
            }
        }

        for constraint in constraints {
            if !constraint(timestamp) {
                return false
            }
        }

        true
    }

    /**
      Tests a date query by running it using the current time
      and ensuring that all values in expected_in are included in the returned
      interval and all in expected_out are not included
    */
    fn test_query(
            query: &str,
            current_time: NaiveDateTime,
            expected_in: Vec<NaiveDateTime>,
            expected_out: Vec<NaiveDateTime>
        ) -> Result<(), String>
    {
        let query_result = parse_date_query(query, &current_time)?;

        for time in expected_in {
            if !timestamp_in_query_result(&time, &query_result) {
                return Err(format!("Timestamp {} was not included in the result of query {}",
                           time,
                           query
                        ))
            }
        }
        for time in expected_out {
            if timestamp_in_query_result(&time, &query_result) {
                return Err(
                    format!("Timestamp {} was included in the result of query {}", time, query)
                )
            }
        }

        Ok(())
    }

    #[test]
    fn modulu_search_test() {
        assert_matches!(test_query(
                "this day",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,09).and_hms(09,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,09,10).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,10,10).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,09).and_hms(12,00,00),
                )
            ), Ok(()));

        assert_matches!(test_query(
                "this month",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,09).and_hms(09,30,36),
                    NaiveDate::from_ymd(2017,09,01).and_hms(09,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,10,10).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,09).and_hms(12,00,00),
                )
            ), Ok(()));

        assert_matches!(test_query(
                "this year",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,09).and_hms(09,30,36),
                    NaiveDate::from_ymd(2017,09,01).and_hms(09,30,36),
                    NaiveDate::from_ymd(2017,07,10).and_hms(19,03,35)
                ),
                vec!(
                    NaiveDate::from_ymd(2016,09,09).and_hms(12,00,00),
                )
            ), Ok(()));

        // First of january 2018 was a monday Started on
        assert_matches!(test_query(
                "this week",
                NaiveDate::from_ymd(2018,01,03).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2018,01,02).and_hms(09,30,36),
                    NaiveDate::from_ymd(2018,01,01).and_hms(09,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,12,31).and_hms(12,00,00),
                    NaiveDate::from_ymd(2018,01,08).and_hms(12,00,00),
                )
            ), Ok(()));
    }

    #[test]
    fn absolute_search_test() {
        assert_matches!(test_query(
                "past day",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,09,09).and_hms(10,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,07,10).and_hms(19,03,35),
                    NaiveDate::from_ymd(2017,09,07).and_hms(19,03,35)
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past week",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,09,09).and_hms(10,30,36),
                    NaiveDate::from_ymd(2017,09,02).and_hms(23,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2016,09,01).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,07,10).and_hms(19,03,35)
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past month",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,09,09).and_hms(10,30,36),
                    NaiveDate::from_ymd(2017,09,02).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,08,10).and_hms(23,30,36),
                ),
                vec!(
                    NaiveDate::from_ymd(2016,08,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,07,10).and_hms(19,03,35)
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past year",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,09,09).and_hms(10,30,36),
                    NaiveDate::from_ymd(2017,09,02).and_hms(23,30,36),
                    NaiveDate::from_ymd(2017,08,10).and_hms(23,30,36),
                    NaiveDate::from_ymd(2016,09,30).and_hms(19,03,35)
                ),
                vec!(
                    NaiveDate::from_ymd(2016,09,08).and_hms(12,00,00),
                )
            ), Ok(()));
    }

    #[test]
    fn date_pattern_query_test() {
        // From a single month
        assert_matches!(test_query(
                "in august",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,08,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,08,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,08,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,20).and_hms(12,00,00)
                ),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,06,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,07,20).and_hms(12,00,00)
                )
            ), Ok(()));

        // From a sepcific day number
        assert_matches!(test_query(
                "on 25",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,06,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,25).and_hms(12,00,00)
                ),
                vec!(
                    NaiveDate::from_ymd(2017,09,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,06,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,07,20).and_hms(12,00,00)
                )
            ), Ok(()));
        //
        // From a sepcific year
        assert_matches!(test_query(
                "in 2017",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,09,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,06,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,09,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,06,20).and_hms(12,00,00),
                ),
                vec!(
                    NaiveDate::from_ymd(2016,09,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,07,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,09,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,25).and_hms(12,00,00)
                )
            ), Ok(()));
    }

    #[test]
    fn since_query_test() {
        // From a single month
        assert_matches!(test_query(
                "since 2017-10-01",
                NaiveDate::from_ymd(2017,12,23).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,10,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,11,20).and_hms(12,00,00)
                ),
                vec!(
                    NaiveDate::from_ymd(2016,08,08).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,09,20).and_hms(12,00,00),
                )
            ), Ok(()));
    }

    #[test]
    fn multi_pattern_query_test() {
        // From a month in a year
        assert_matches!(test_query(
                "in august 2017",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,08,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,08,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,08,08).and_hms(12,00,00),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,09,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,07,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,08,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,25).and_hms(12,00,00)
                )
            ), Ok(()));

        // From a specific date any year
        assert_matches!(test_query(
                "on august 26",
                NaiveDate::from_ymd(2017,09,09).and_hms(12,00,00),
                vec!(
                    NaiveDate::from_ymd(2017,08,26).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,08,26).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,26).and_hms(12,00,00),
                ),
                vec!(
                    NaiveDate::from_ymd(2017,09,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2017,07,20).and_hms(12,00,00),
                    NaiveDate::from_ymd(2016,08,25).and_hms(12,00,00),
                    NaiveDate::from_ymd(2015,08,25).and_hms(12,00,00)
                )
            ), Ok(()));
    }
}
