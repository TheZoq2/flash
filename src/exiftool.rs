use std::collections::HashMap;

extern crate regex;

use self::regex::Regex;

use std;


#[derive(Clone, Debug)]
pub enum GpsStringParseError
{
    NumberParseError,
    InvalidDirection(String),
    BadFormat
}
impl std::convert::From<std::num::ParseFloatError> for GpsStringParseError
{
    fn from(_: std::num::ParseFloatError) -> GpsStringParseError
    {
        GpsStringParseError::NumberParseError
    }
}
impl std::convert::From<std::num::ParseIntError> for GpsStringParseError
{
    fn from(_: std::num::ParseIntError) -> GpsStringParseError
    {
        GpsStringParseError::NumberParseError
    }
}

#[derive(Clone, Debug)]
enum ExifError
{
    InvalidGpsCoordinate(GpsStringParseError)
}


#[derive(PartialEq, Debug)]
pub enum CardinalDirection
{
    East,
    West,
    North,
    South
}

impl CardinalDirection
{
    fn from_str(name: &str) -> Result<CardinalDirection, GpsStringParseError>
    {
        match name
        {
            "N" => Ok(CardinalDirection::North),
            "S" => Ok(CardinalDirection::South),
            "W" => Ok(CardinalDirection::West),
            "E" => Ok(CardinalDirection::East),
            other => Err(GpsStringParseError::InvalidDirection(String::from(other))),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct GpsCoordinate
{
    degrees: i16,
    minutes: i16,
    seconds: f32,
    direction: CardinalDirection
}

impl GpsCoordinate
{
    pub fn from_str(string: &str) -> Result<GpsCoordinate, GpsStringParseError>
    {
        lazy_static! {
            static ref RE: Regex = Regex::new("(\\d*) deg (\\d*)' (\\d*.?\\d*)\" ([NEWS])").unwrap();
        }

        match RE.captures_iter(string).next()
        {
            Some(val) => Ok(GpsCoordinate{
                degrees: val[1].parse()?,
                minutes: val[2].parse()?,
                seconds: val[3].parse()?,
                direction: CardinalDirection::from_str(&val[4])?
            }),
            None => Err(GpsStringParseError::BadFormat)
        }
    }

    pub fn new(degrees: i16, minutes: i16, seconds: f32, direction: CardinalDirection) -> GpsCoordinate
    {
        GpsCoordinate{
            degrees: degrees,
            minutes: minutes,
            seconds: seconds,
            direction: direction
        }
    }
}

/**
  A GPS location
 */
pub struct Location
{
    longitude: GpsCoordinate,
    latitude: GpsCoordinate
}
impl Location
{
    pub fn new(longitude: GpsCoordinate, latitude: GpsCoordinate) -> Location
    {
        Location {
            longitude: longitude,
            latitude: latitude
        }
    }
}

pub struct ExifData
{
    tags: HashMap<String, String>
}

impl ExifData
{
    pub fn from_exiftool_string(data: &str) -> Result<ExifData, String>
    {
        let mut result = ExifData{
            tags: HashMap::new()
        };

        lazy_static! {
            static ref DATA_REGEX: Regex = Regex::new(r"(.*\b)\s*: (.*)").unwrap();
        }

        for matches in DATA_REGEX.captures_iter(data)
        {
            //TODO: Handle erros here
            result.tags.insert(String::from(&matches[1]), String::from(&matches[2]));
        }

        Ok(result)
    }

    pub fn get_tag(&self, name: &str) -> Option<&str>
    {
        match self.tags.get(name)
        {
            Some(tag) => Some(&tag),
            None => None
        }
    }

    //pub fn get_timestamp

    /*
    pub fn get_location(&self) -> Option<Location>
    {
        match (self.get_tag("GPS Latitude"), self.get_tag("GPS Longitude"))
        {
            (Some(latitude), Some(longitude)) => 
                Some(Location::new(
                        GpsCoordinate::from_str(latitude),
                        GpsCoordinate::from_str(longitude)
                    )),
            (_, _) => None
        }
    }
    */
}


#[cfg(test)]
mod exif_data_tests
{
    use super::*;

    #[test]
    fn well_formed_file()
    {
        let file_content = include_str!("../test/files/exif1.txt");

        let data = ExifData::from_exiftool_string(file_content).unwrap();

        assert_eq!(data.get_tag("GPS Img Direction"), Some("330"));
        assert_eq!(data.get_tag("X Resolution"), Some("72"));
        assert_eq!(data.get_tag("Non-existing tag"), None);
    }

    #[test]
    fn gps_coordinate_test()
    {
        assert_eq!(
            GpsCoordinate::from_str("58 deg 28' 5.45\" N").unwrap(), 
            GpsCoordinate::new(58, 28, 5.45, CardinalDirection::North)
        );
        assert_eq!(
            GpsCoordinate::from_str("58 deg 28' 5.45\" S").unwrap(), 
            GpsCoordinate::new(58, 28, 5.45, CardinalDirection::South)
        );
    }
}
