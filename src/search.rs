//#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

use std::borrow::Cow;

const TAG_LIST_REGEX: &str = r"of (?P<list>[\w[:blank:],]+);{0,1}";

/**
  Returns all the tags specified in a search query
*/
pub fn get_tags_from_query(query: &str) -> (Vec<String>, Vec<String>)
{
    lazy_static!{
        static ref TAG_RE: Regex = Regex::new(r"\w+").unwrap();
    }

    let list_string = match get_tag_list_from_query(query)
    {
        Some(val) => val,
        None => return (vec!(), vec!())
    };

    let tag_vec = get_tags_from_list_string(&list_string);

    separate_negated_tags(&tag_vec)
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
    Some(AND_RE.replace_all(list_str, ", "))
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

    TAG_RE.captures_iter(list_string)
        .filter_map(|capture| {
            capture.name("tag").map(|val| Cow::from(val.as_str()))
        })
        .collect()
}

/**
  Separates a list of possibly negated tags 'not <tag>' into a list
  of non-negated tags and a list of negated tags

  returns (non-negated, negated)
*/
fn separate_negated_tags(tags: &[Cow<str>]) -> (Vec<String>, Vec<String>)
{
    lazy_static! {
        static ref NEGATED_REGEX: Regex =Regex::new(r"not (?P<tag>.+)").unwrap();
    };

    tags.iter()
        .fold((vec!(), vec!()), |(mut non_negated, mut negated), tag| {
            let captures = NEGATED_REGEX.captures(tag);
            match captures {
                // We can be sure that 1 exists since the capture grouop must be matched
                // Therefore unwrap is safe
                Some(captures) => negated.push(String::from(captures.get(1).unwrap().as_str())),
                None => non_negated.push(tag.to_string())
            }

            (non_negated, negated)
        })
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

    if let Some(cap) = TIME_INTERVAL_REGEX.captures(query)
    {
        let descriptor = cap.get(1).unwrap().as_str();
        let time = cap.get(2).unwrap().as_str();
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
        assert_eq!(get_tags_from_query("of things and stuff"), 
                   (mapvec!(String::from: "things", "stuff"), vec!()));
        assert_eq!(get_tags_from_query("of things"),
                   (mapvec!(String::from: "things"), vec!()));
        assert_eq!(get_tags_from_query("of things, stuff and items"),
                   (mapvec!(String::from: "things", "stuff", "items"), vec!()));
    }

    #[test]
    fn no_tags_should_return_empty_vector()
    {
        assert_eq!(get_tags_from_query("of"), (vec!(), vec!()));
        assert_eq!(get_tags_from_query(""), (vec!(), vec!()));
        assert_eq!(get_tags_from_query("in linköping"), (vec!(), vec!()));
        assert_eq!(get_tags_from_query("from this year"), (vec!(), vec!()));
    }

    #[test]
    fn more_things_specified_should_give_correct_tags()
    {
        assert_eq!(get_tags_from_query("of things and stuff; from last year"), 
                   (mapvec!(String::from: "things", "stuff"), vec!()));
        assert_eq!(get_tags_from_query("of things and stuff ;from last year in linköping"),
                   (mapvec!(String::from: "things", "stuff"), vec!()));
    }

    #[test]
    fn searching_for_not_tags_should_work()
    {
        assert_eq!(get_tags_from_query("of things and not stuff"),
                (mapvec!(String::from: "things"), mapvec!(String::from: "stuff")));
    }
}

#[cfg(test)]
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

    fn tag_negation_tests()
    {
        assert_eq!(separate_negated_tags(&mapvec!(Cow::from: "yolo", "not swag")),
                (mapvec!(String::from: "yolo"), mapvec!(String::from: "swag")));
    }

    /**
      Tries to replicate a bug where searching for negated tags would not propperly negate them
    */
    #[test]
    fn negation_bug_test()
    {
        let search_string = "of not snödroppe";

        let tag_list = get_tag_list_from_query(search_string).unwrap();
        assert_eq!(tag_list, Cow::from("not snödroppe"));

        let tags = get_tags_from_list_string(&tag_list);
        assert_eq!(tags, mapvec!(Cow::from: "not snödroppe"));

        let (tags, negated) = separate_negated_tags(&tags);

        assert_eq!(tags, vec!());
        assert_eq!(negated, mapvec!(String::from: "snödroppe"));
    }
}
