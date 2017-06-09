//#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

use std::borrow::Cow;

const TAG_LIST_REGEX: &str = r"of (?P<list>[\w[:blank:],]+);{0,1}";

/**
  Returns all the tags specified in a search query
*/
pub fn get_tags_from_query(query: &str) -> Vec<String>
{
    lazy_static!{
        static ref TAG_RE: Regex = Regex::new(r"\w+").unwrap();
    }

    let list_string = match get_tag_list_from_query(query)
    {
        Some(val) => val,
        None => return vec!()
    };

    get_tags_from_list_string(&list_string)
        .iter()
        .map(|cow| String::from(cow.clone()))
        .collect()
}

/**
  Takes a string on the form 
  "... of <tag1>, <tag2>, ... [,|and] <tagn>; ..."
  and returns a string containing all the tags separated by ,
*/
fn get_tag_list_from_query(query: &str) -> Option<Cow<str>>
{
    lazy_static!{
        static ref AND_RE: Regex = Regex::new(r"\Wand\W").unwrap();
        static ref TAG_LIST_RE: Regex = Regex::new(TAG_LIST_REGEX).unwrap();
    }

    // Try to match the search string with the tag list regex template
    // and find the list group
    let captures = match TAG_LIST_RE.captures(query)
    {
        None => return None,
        Some(v) => v
    };

    // Separate the actual list of tags
    let list_str = match captures.name("list")
    {
        Some(v) => v.as_str(),
        None => {return None}
    };

    // Replace 'and' with ','
    Some(AND_RE.replace_all(&list_str, ", "))
}

/**
  Takes a string of tags separated by commas and optionally by whitespace
  and returns a vector the tags. The tags can be on the form <name> or <not name>
*/
fn get_tags_from_list_string(list_string: &str) -> Vec<Cow<str>>
{
    lazy_static! {
        static ref TAG_RE: Regex =
            Regex::new(r"[[:blank:]]*(?P<tag>\w[\w[:blank:]]*\w)[,[:blank:]]*")
            .unwrap();
    };

    TAG_RE.captures_iter(&list_string)
        .filter_map(|capture| {
            capture.name("tag").map(|val| Cow::from(val.as_str()))
        })
        .collect()
}

fn get_positive_and_negative_tags(unparsed_tags: &Vec<Cow<str>>) -> (Vec<Cow<str>>, Vec<Cow<str>>)
{

}


/**
  A set of times to include in a search
*/
pub enum Time {
    Interval(u32, u32),
}

pub fn get_time_from_query(query: &str) -> Time
{
    lazy_static!{
        static ref TIME_INTERVAL_REGEX: Regex = Regex::new(r"from (this|last|the past) (\w*)").unwrap();
    }

    match TIME_INTERVAL_REGEX.captures(query)
    {
        Some(cap) => 
        {
            let descriptor = cap.get(1).unwrap().as_str();
            let time = cap.get(2).unwrap().as_str();
        },
        None =>
        {

        }
    }

    unimplemented!()
}

#[cfg(test)]
mod public_query_tests
{
    use super::*;

    #[test]
    fn query_with_only_tags()
    {
        assert_eq!(get_tags_from_query("of things and stuff"), mapvec!(String::from: "things", "stuff"));
        assert_eq!(get_tags_from_query("of things"), mapvec!(String::from: "things"));
        assert_eq!(get_tags_from_query("of things, stuff and items"),
                mapvec!(String::from: "things", "stuff", "items"));
    }

    #[test]
    fn no_tags_should_return_empty_vector()
    {
        assert_eq!(get_tags_from_query("of"), Vec::<String>::new());
        assert_eq!(get_tags_from_query(""), Vec::<String>::new());
        assert_eq!(get_tags_from_query("in linköping"), Vec::<String>::new());
        assert_eq!(get_tags_from_query("from this year"), Vec::<String>::new());
    }

    #[test]
    fn more_things_specified_should_give_correct_tags()
    {
        assert_eq!(get_tags_from_query("of things and stuff; from last year"), 
                   mapvec!(String::from: "things", "stuff"));
        assert_eq!(get_tags_from_query("of things and stuff ;from last year in linköping"),
                   mapvec!(String::from: "things", "stuff"));
    }

    #[test]
    fn searching_for_not_tags_should_work()
    {
        assert_eq!(get_tags_from_query("of things and not stuff"),
                mapvec!(String::from: "things", "not stuff"));
    }
}

mod private_query_tests
{
    use super::*;

    #[test]
    fn tag_list_from_query_tests()
    {
        // Simple, tags only string
        assert_eq!(get_tag_list_from_query("of things, stuff, yolo and swag"),
                   Some(Cow::from("things, stuff, yolo, swag")));
        // Tags with spaces
        assert_eq!(get_tag_list_from_query("of many things,     stuff, yolo swag and swag"),
                   Some(Cow::from("many things,     stuff, yolo swag, swag")));

        // Other data before 'of'
        assert_eq!(get_tag_list_from_query("from today of things and stuff"),
                  Some(Cow::from("things, stuff")));

        // Other data after ';'
        assert_eq!(get_tag_list_from_query("of things and stuff; from today"),
                   Some(Cow::from("things, stuff")));

        // No tags specified
        assert_eq!(get_tag_list_from_query("from today; in linköping"), None);

        //No tags specified but the list is empty
        assert_eq!(get_tag_list_from_query("of ;"), None);
    }

    fn tag_from_list_string_tests()
    {
        //Simple tags, no whitespaces
        assert_eq!(get_tags_from_list_string("some,thing,yolo"),
                  mapvec!(Cow::from: "some", "thing", "yolo"));

        //Whitespace
        assert_eq!(get_tags_from_list_string("not some,  thing   , not yo lo "),
                  mapvec!(Cow::from: "not some", "thing", "not yo lo"));
    }
}
