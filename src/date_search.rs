use chrono::{NaiveDateTime, NaiveDate, NaiveTime, Datelike, Duration};

use std::vec::Vec;
use std::str::{FromStr, SplitWhitespace};

#[derive(Debug)]
pub enum TimeParseError {
    UnexpectedWord(String),
    UnexpectedEndOfQuery
}

pub struct Interval {
    start: NaiveDateTime,
    end: NaiveDateTime
}

impl Interval {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime) -> Interval {
        Interval {
            start,
            end
        }
    }

    pub fn contains(&self, time: &NaiveDateTime) -> bool {
        if time >= &self.start && time < &self.end{
            true
        }
        else {
            false
        }
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

pub struct TimeSearchResult {
    intervals: Vec<(NaiveDateTime, NaiveDateTime)>,
    constraints: Vec<Box<DateConstraintFunction>>
}

pub fn parse_date_query(query: &str, current_time: &NaiveDateTime)
    -> Result<(Vec<Interval>, Vec<Box<DateConstraintFunction>>), TimeParseError>
{
    let mut words = query.split_whitespace();

    match words.next() {
        Some("this") => Ok((parse_modulu_search(&mut words, current_time)?, vec!())),
        Some("past") => Ok((parse_absolute_search(&mut words, current_time)?, vec!())),
        Some("in") | Some("on") => Ok((vec!(), parse_date_pattern_search(&mut words)?)),
        Some("between") => unimplemented!(),
        // Special keywords, or unexpected tokens
        Some(other) => unimplemented!(),
        None => Err(TimeParseError::UnexpectedEndOfQuery)
    }
}


fn parse_modulu_search(query: &mut SplitWhitespace, current_time: &NaiveDateTime)
    -> Result<Vec<Interval>, TimeParseError>
{
    let time_descriptor = match query.next() {
        Some(word) => TimeDescriptor::from_str(&word)?,
        None => return Err(TimeParseError::UnexpectedEndOfQuery)
    };

    let start_date = match time_descriptor {
        TimeDescriptor::Day => current_time.date(),
        TimeDescriptor::Week => unimplemented!("NaiveDateTime does not have a week concept"),
        TimeDescriptor::Month => current_time.date().with_day0(0).unwrap(),
        TimeDescriptor::Year => current_time.date().with_day0(0)
                .unwrap()
                .with_month0(0).
                unwrap()
    };

    let start = NaiveDateTime::new(start_date, NaiveTime::from_hms_milli(0,0,0,0));

    Ok(vec!(Interval::new(start, current_time.clone())))
}

fn parse_absolute_search(query: &mut SplitWhitespace, current_time: &NaiveDateTime) 
    -> Result<Vec<Interval>, TimeParseError>
{
    let time_descriptor = match query.next() {
        Some(word) => TimeDescriptor::from_str(&word)?,
        None => return Err(TimeParseError::UnexpectedEndOfQuery)
    };

    let subtracted_duration = match time_descriptor {
        TimeDescriptor::Day => Duration::days(1),
        TimeDescriptor::Week => Duration::weeks(1),
        TimeDescriptor::Month => Duration::days(30),
        TimeDescriptor::Year => Duration::days(365)
    };

    Ok(vec!(Interval::new(*current_time - subtracted_duration, current_time.clone())))
}


fn parse_date_pattern_search(query: &mut SplitWhitespace)
    -> Result<Vec<Box<DateConstraintFunction>>, TimeParseError>
{
    let mut result_functions: Vec<Box<DateConstraintFunction>>= vec!();

    for word in query {
        if let Ok(month) = Month::from_str(word) {
            result_functions.push(
                    Box::new(move |date| date.month0() == month.as_number0())
                )
        }
        else if let Ok(number) = word.parse::<u32>() {
            if number < 31 {
                result_functions.push(
                        Box::new(move |date| date.day() == number)
                    )
            }
            else {
                result_functions.push(
                        Box::new(move |date| date.year() == number as i32)
                    )
            }
        }
    }

    Ok(result_functions)
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
            query_result: &(Vec<Interval>, Vec<Box<DateConstraintFunction>>)
        ) -> bool
    {
        let (ref intervals, ref constraints) = *query_result;

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
            current_time: &str,
            expected_in: Vec<&str>,
            expected_out: Vec<&str>
        ) -> Result<(), String>
    {
        let current_time = NaiveDateTime::parse_from_str(current_time, "%Y-%m-%d %H:%M:%S")
                .unwrap();

        let query_result = parse_date_query(query, &current_time)?;

        let expected_in: Vec<NaiveDateTime> = expected_in.iter()
                .map(|val| NaiveDateTime::parse_from_str(val, "%Y-%m-%d %H:%M:%S").unwrap())
                .collect();
        let expected_out: Vec<NaiveDateTime> = expected_out.iter()
                .map(|val| NaiveDateTime::parse_from_str(val, "%Y-%m-%d %H:%M:%S").unwrap())
                .collect();

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
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-09 09:30:36",
                ),
                vec!(
                    "2017-09-10 12:00:00",
                    "2017-10-10 12:00:00",
                    "2016-09-09 12:00:00",
                )
            ), Ok(()));

        assert_matches!(test_query(
                "this month",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-09 09:30:36",
                    "2017-09-01 09:30:36",
                ),
                vec!(
                    "2017-10-10 12:00:00",
                    "2016-09-09 12:00:00",
                )
            ), Ok(()));

        assert_matches!(test_query(
                "this year",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-09 09:30:36",
                    "2017-09-01 09:30:36",
                    "2017-07-10 19:03:35"
                ),
                vec!(
                    "2016-09-09 12:00:00",
                )
            ), Ok(()));
    }

    #[test]
    fn absolute_search_test() {
        assert_matches!(test_query(
                "past day",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-08 23:30:36",
                    "2017-09-09 10:30:36",
                ),
                vec!(
                    "2017-07-10 19:03:35",
                    "2017-09-07 19:03:35"
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past week",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-08 23:30:36",
                    "2017-09-09 10:30:36",
                    "2017-09-02 23:30:36",
                ),
                vec!(
                    "2016-09-01 12:00:00",
                    "2017-07-10 19:03:35"
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past month",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-08 23:30:36",
                    "2017-09-09 10:30:36",
                    "2017-09-02 23:30:36",
                    "2017-08-10 23:30:36",
                ),
                vec!(
                    "2016-08-08 12:00:00",
                    "2017-07-10 19:03:35"
                )
            ), Ok(()));

        assert_matches!(test_query(
                "past year",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-08 23:30:36",
                    "2017-09-09 10:30:36",
                    "2017-09-02 23:30:36",
                    "2017-08-10 23:30:36",
                    "2016-09-30 19:03:35"
                ),
                vec!(
                    "2016-09-08 12:00:00",
                )
            ), Ok(()));
    }

    #[test]
    fn date_pattern_query_test() {
        // From a single month
        assert_matches!(test_query(
                "in august",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-08-08 12:00:00",
                    "2017-08-20 12:00:00",
                    "2016-08-20 12:00:00",
                    "2015-08-20 12:00:00"
                ),
                vec!(
                    "2017-09-08 12:00:00",
                    "2017-06-20 12:00:00",
                    "2016-09-20 12:00:00",
                    "2015-07-20 12:00:00"
                )
            ), Ok(()));

        // From a sepcific day number
        assert_matches!(test_query(
                "on 25",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-25 12:00:00",
                    "2017-06-25 12:00:00",
                    "2016-09-25 12:00:00",
                    "2015-08-25 12:00:00"
                ),
                vec!(
                    "2017-09-08 12:00:00",
                    "2017-06-20 12:00:00",
                    "2016-09-20 12:00:00",
                    "2015-07-20 12:00:00"
                )
            ), Ok(()));
        //
        // From a sepcific year
        assert_matches!(test_query(
                "in 2017",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-09-25 12:00:00",
                    "2017-06-25 12:00:00",
                    "2017-09-08 12:00:00",
                    "2017-06-20 12:00:00",
                ),
                vec!(
                    "2016-09-20 12:00:00",
                    "2015-07-20 12:00:00",
                    "2016-09-25 12:00:00",
                    "2015-08-25 12:00:00"
                )
            ), Ok(()));
    }

    #[test]
    fn multi_pattern_query_test() {
        // From a month in a year
        assert_matches!(test_query(
                "in august 2017",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-08-25 12:00:00",
                    "2017-08-25 12:00:00",
                    "2017-08-08 12:00:00",
                ),
                vec!(
                    "2017-09-20 12:00:00",
                    "2017-07-20 12:00:00",
                    "2016-08-25 12:00:00",
                    "2015-08-25 12:00:00"
                )
            ), Ok(()));

        // From a specific date any year
        assert_matches!(test_query(
                "on august 26",
                "2017-09-09 12:00:00",
                vec!(
                    "2017-08-26 12:00:00",
                    "2016-08-26 12:00:00",
                    "2015-08-26 12:00:00",
                ),
                vec!(
                    "2017-09-20 12:00:00",
                    "2017-07-20 12:00:00",
                    "2016-08-25 12:00:00",
                    "2015-08-25 12:00:00"
                )
            ), Ok(()));
    }
}
