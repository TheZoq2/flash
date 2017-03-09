use std::collections::HashMap;

extern crate regex;

use self::regex::Regex;

struct ExifData
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
}
