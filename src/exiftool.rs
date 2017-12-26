use std::collections::HashMap;

extern crate regex;
extern crate chrono;

use std::str::FromStr;

use self::regex::Regex;

use std::process::Command;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Utf8(::std::string::FromUtf8Error);
    }

    errors {
        NoSuchTag(name: String) {
            description("The file did not contain the specified tag")
            display("File did not contain tag '{}'", name)
        }
        MalformedDatetime(data: String) {
            description("The date string was malformed")
            display("Unexpected date format in exif data: {}", data)
        }
        MalformedUtf8 {
            description("Exiftool returned invalid UTF-8")
            display("Invalid UTF-8 returned from exiftool")
        }
    }
}


#[derive(Debug)]
pub struct ExifData {
    tags: HashMap<String, String>,
}

const DATE_FORMAT: &'static str = "%Y:%m:%d %H:%M:%S.%e";

impl ExifData {
    pub fn from_exiftool_string(data: &str) -> Result<ExifData> {
        let mut result = ExifData {
            tags: HashMap::new(),
        };

        lazy_static! {
            static ref DATA_REGEX: Regex = Regex::new(r"(.*\b)\s*: (.*)").unwrap();
        }

        for matches in DATA_REGEX.captures_iter(data) {
            //TODO: Handle erros here
            result
                .tags
                .insert(String::from(&matches[1]), String::from(&matches[2]));
        }

        Ok(result)
    }

    pub fn from_file(file: &str) -> Result<ExifData> {
        let mut cmd = Command::new("exiftool");
        cmd.arg("-d");
        cmd.arg(DATE_FORMAT);
        cmd.arg(file);

        let command_output = {
            let raw = cmd.output()?.stdout;

            String::from_utf8(raw)?
        };

        Self::from_exiftool_string(&command_output)
    }

    pub fn get_tag(&self, name: &str) -> Option<&str> {
        match self.tags.get(name) {
            Some(tag) => Some(tag),
            None => None,
        }
    }

    pub fn get_creation_date(&self) -> Result<chrono::NaiveDateTime> {
        let target_tag = "Date/Time Original";
        match self.get_tag(target_tag) {
            Some(date_string) => {
                let parsed = chrono::NaiveDateTime::parse_from_str(date_string, DATE_FORMAT);

                match parsed {
                    Ok(result) => Ok(result),
                    _ => Err(ErrorKind::MalformedDatetime(String::from(date_string)).into()),
                }
            }
            None => Err(ErrorKind::NoSuchTag(String::from(target_tag)).into()),
        }
    }
}


#[cfg(test)]
mod exif_data_tests {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn well_formed_file() {
        let file_content = include_str!("../test/files/exif1.txt");

        let data = ExifData::from_exiftool_string(file_content).unwrap();

        //assert_eq!(data.get_tag("GPS Img Direction"), Some("330"));
        assert_eq!(data.get_tag("X Resolution"), Some("72"));
        assert_eq!(data.get_tag("Create Date"), Some("2017:09:11 14:40:00.11"));
        assert_eq!(data.get_tag("Non-existing tag"), None);

        let expected_date = chrono::NaiveDate::from_ymd(2017, 9, 11).and_hms(14, 40, 0);
        assert_eq!(data.get_creation_date().unwrap(), expected_date);
    }



    #[test]
    fn read_exif_from_file()
    {
        let filename = "test/media/DSC_0001.JPG";

        let data = ExifData::from_file(filename).unwrap();

        assert_eq!(data.get_tag("Image Width"), Some("6000"));

        let expected_date = chrono::NaiveDate::from_ymd(2016, 12, 16).and_hms(21, 34, 26);
        assert_eq!(data.get_creation_date().unwrap(), expected_date);
    }

    #[test]
    fn creation_date_from_oneplus() {
        let filename = "test/media/IMG_20171024_180300.jpg";

        let data = ExifData::from_file(filename).unwrap();

        let expected_date = chrono::NaiveDate::from_ymd(2017, 10, 24).and_hms(18, 3, 00);
        assert_eq!(data.get_creation_date().unwrap(), expected_date);
    }
}
