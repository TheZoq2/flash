//#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

use file_database::FileEntry;

const TAG_LIST_REGEX: &str = r"of (?P'list'(not ){0,1}\w*(, (not ){0,1}\w*)*( and (not ){0,1}\w*){0,1})";

/**
  Returns true if the specified file has all of the specified tags
*/
fn has_all_tags(file: &FileEntry, tags: Vec<String>) -> bool
{
    for tag in tags
    {
        if !file.has_tag(tag)
        {
            return false;
        }
    }
    true
}

/**
  Returns true if the specified file has none 
*/
fn has_some_tag(file: &FileEntry, tags: Vec<String>) -> bool
{
    for tag in tags
    {
        //if has_tag()
    }

    unimplemented!()
}



pub fn get_tags_from_query(query: &str) -> Vec<String>
{
    lazy_static!{
        static ref RE: Regex = Regex::new(TAG_LIST_REGEX).unwrap();
    }

    //Try to match the search string with the tag list regex template
    //and find the list group
    let captures = match  RE.captures(query)
    {
        None => return vec!(),
        Some(v) => v
    };

    let list_str = match captures.name("list")
    {
        Some(v) => v,
        None => return vec!()
    };

    vec!(String::from(list_str))
}


#[cfg(test)]
mod query_tests
{
    use super::*;

    #[test]
    fn query_with_only_tags()
    {
        assert_eq!(get_tags_from_query("of things and stuff"), vec!("things and stuff"));
    }
}
