use std::collections::HashMap;

extern crate regex;

use self::regex::Regex;


enum ExifError
{
    InvalidGpsCoordinate(String)
}



pub enum CardinalDirection
{
    East,
    West,
    North,
    South
}

impl CardinalDirection
{
    fn from_str(name: &str) -> Result<CardinalDirection, String>
    {
        match name
        {
            "N" => Ok(CardinalDirection::North),
            "S" => Ok(CardinalDirection::South),
            "W" => Ok(CardinalDirection::West),
            "E" => Ok(CardinalDirection::East),
            other => Err(format!("{} is not a valid direction", other)),
        }
    }
}

pub struct GpsCoordinate
{
    degrees: i16,
    minutes: i16,
    seconds: f32,
    direction: CardinalDirection
}

impl GpsCoordinate
{
    pub fn from_str(string: &str) -> Result<GpsCoordinate, ExifError>
    {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(.*\b)\s*: (.*)").unwrap();
        }

        match RE.captures_iter(string).next()
        {
            Some(val) => Ok(GpsCoordinate{
                degrees: val[1].parse()?,
                minutes: val[2].parse()?,
                seconds: val[3].parse()?,
                direction: CardinalDirection::from_str(&val[4])?
            }),
            None => Err(format!("String {} is not a valid GPS string", string))
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

    pub fn get_location()
    {
        
    }
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
        
    }
}
