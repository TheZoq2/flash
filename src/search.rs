//#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;

use std::borrow::Cow;

use chrono::{NaiveDateTime, Utc};

use date_search::{DateConstraints, parse_date_query};

use util;

#[derive(Debug)]
pub struct SavedSearchQuery {
    pub tags: Vec<String>,
    pub negated_tags: Vec<String>,
    pub date_constraints: DateConstraints,
}

impl SavedSearchQuery {
    pub fn empty() -> Self {
        Self {
            tags: vec!(),
            negated_tags: vec!(),
            date_constraints: DateConstraints::empty()
        }
    }

    pub fn with_tags((tags, negated_tags): (Vec<String>, Vec<String>)) -> Self {
        Self {
            tags,
            negated_tags,
            .. Self::empty()
        }
    }

    pub fn with_date_constraints(date_constraints: DateConstraints) -> Self {
        Self {
            date_constraints,
            .. Self::empty()
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            date_constraints: self.date_constraints.merge(&other.date_constraints),
            tags: util::merge_vectors(&self.tags, &other.tags),
            negated_tags: util::merge_vectors(&self.negated_tags, &other.negated_tags),
        }
    }
}

/**
  The type of a search, it could either be a search for previously
  saved files in the database, orater for new files at a specified path
*/
#[derive(Debug)]
pub enum SearchType {
    Path(String),
    Saved(SavedSearchQuery),
}

/**
  Parses a search query to determine what the user searched for
*/
pub fn parse_search_query(query: &str) -> SearchType {
    lazy_static! {
        static ref PATH_RE: Regex = Regex::new(r"^/.*").unwrap();
        static ref QUERY_SECTION_REGEX: Regex = 
                Regex::new(r"(?P<type>of|from) (?P<main>.+?)(;|$)").unwrap();
    }

    if PATH_RE.is_match(query) {
        // Strip the first /
        SearchType::Path(query[1..].to_owned())
    }
    else {
        let query_captures = QUERY_SECTION_REGEX.captures_iter(query);

        let query = query_captures.map(|cap| {
                let type_str = cap.name("type").map(|x| x.as_str());
                let content_str = cap.name("main").map(|x| x.as_str());

                query_section_type(type_str, content_str)
            })
            .fold(SavedSearchQuery::empty(), |prev, query| {
                let new = match query {
                    QuerySectionType::Tags(tag_list) =>
                        SavedSearchQuery::with_tags(tags_from_tag_list_string(&tag_list)),
                    QuerySectionType::Time(time_str) =>
                            SavedSearchQuery::with_date_constraints(
                                get_date_constraints_from_string(&time_str)
                            )
                };

                prev.merge(&new)
            });

        SearchType::Saved(query)
    }
}

enum QuerySectionType {
    Tags(String),
    Time(String)
}

fn query_section_type(type_str: Option<&str>, content_str: Option<&str>)
        -> QuerySectionType
{
    if let (Some(type_str), Some(content_str)) = (type_str, content_str) {
        match type_str {
            "from" => QuerySectionType::Time(content_str.to_owned()),
            "of" => QuerySectionType::Tags(content_str.to_owned()),
            _ => {
                panic!("The string matched the regex but the type was not correct");
            }
        }
    }
    else {
        panic!("The string matched the regex, but the expected groups were not part of the match");
    }
}



fn get_date_constraints_from_string(query: &str) -> DateConstraints {
    let current_time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
    parse_date_query(query, &current_time)
            .unwrap_or_else(|_| DateConstraints::empty())
}

fn tags_from_tag_list_string(list_string: &str) -> (Vec<String>, Vec<String>) {
    let clean_string = clean_tag_list_string(list_string);

    let tags = get_tags_from_list_string(&clean_string);

    separate_negated_tags(&tags)
}

/**
  Takes a string on the form
  "... of <tag1>, <tag2>, ... [,|and] <tagn>; ..."
  and returns a string containing all the tags separated by ,
*/
fn clean_tag_list_string(list_str: &str) -> Cow<str> {
    lazy_static!{
        static ref AND_RE: Regex = Regex::new(r"\Wand\W").unwrap();
    }

    // Replace 'and' with ','
    AND_RE.replace_all(list_str, ", ")
}

/**
  Takes a string of tags separated by commas and optionally by whitespace
  and returns a vector the tags. The tags can be on the form <name> or <not name>
*/
fn get_tags_from_list_string(list_string: &str) -> Vec<Cow<str>> {
    lazy_static! {
        static ref TAG_RE: Regex =
            Regex::new(r"[[:blank:]]*(?P<tag>\w[\w[:blank:]]*\w)[,[:blank:]]*")
            .unwrap();
    };

    TAG_RE
        .captures_iter(list_string)
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
fn separate_negated_tags(tags: &[Cow<str>]) -> (Vec<String>, Vec<String>) {
    lazy_static! {
        static ref NEGATED_REGEX: Regex =Regex::new(r"not (?P<tag>.+)").unwrap();
    };

    tags.iter().fold(
        (vec![], vec![]),
        |(mut non_negated, mut negated), tag| {
            let captures = NEGATED_REGEX.captures(tag);
            match captures {
                // We can be sure that 1 exists since the capture grouop must be matched
                // Therefore unwrap is safe
                Some(captures) => negated.push(String::from(captures.get(1).unwrap().as_str())),
                None => non_negated.push(tag.to_string()),
            }

            (non_negated, negated)
        },
    )
}



#[cfg(test)]
fn get_tags_from_query(query: &str) -> (Vec<String>, Vec<String>) {
    let query_result = parse_search_query(query);

    if let SearchType::Saved(query_result) = query_result {
        (query_result.tags, query_result.negated_tags)
    }
    else {
        panic!("Expected search to contain tags, but it was not a search for Saved files");
    }
}

#[cfg(test)]
mod public_query_tests {
    use super::*;

    #[test]
    fn query_with_only_tags() {
        assert_eq!(
            get_tags_from_query("of things and stuff"),
            (mapvec!(String::from: "things", "stuff"), vec![])
        );
        assert_eq!(
            get_tags_from_query("of things"),
            (mapvec!(String::from: "things"), vec![])
        );
        assert_eq!(
            get_tags_from_query("of things, stuff and items"),
            (mapvec!(String::from: "things", "stuff", "items"), vec![])
        );
    }

    #[test]
    fn no_tags_should_return_empty_vector() {
        assert_eq!(get_tags_from_query("of"), (vec![], vec![]));
        assert_eq!(get_tags_from_query(""), (vec![], vec![]));
        assert_eq!(get_tags_from_query("in linköping"), (vec![], vec![]));
        assert_eq!(get_tags_from_query("from this year"), (vec![], vec![]));
    }

    #[test]
    fn more_things_specified_should_give_correct_tags() {
        assert_eq!(
            get_tags_from_query("of things and stuff; from past year"),
            (mapvec!(String::from: "things", "stuff"), vec![])
        );
        assert_eq!(
            get_tags_from_query("of things and stuff ;from past year; in linköping"),
            (mapvec!(String::from: "things", "stuff"), vec![])
        );
    }

    #[test]
    fn searching_for_not_tags_should_work() {
        assert_eq!(
            get_tags_from_query("of things and not stuff"),
            (
                mapvec!(String::from: "things"),
                mapvec!(String::from: "stuff")
            )
        );
    }

    #[test]
    fn searching_for_directories_should_work() {
        assert_matches!(parse_search_query("/something/other"), SearchType::Path(_));
        assert_matches!(parse_search_query("/something/folder with space"), SearchType::Path(_));
        assert_matches!(parse_search_query("/other"), SearchType::Path(_));
        assert_matches!(parse_search_query("/"), SearchType::Path(_));
        assert_matches!(parse_search_query(" /"), SearchType::Saved(_));
        assert_matches!(parse_search_query("of things/stuff"), SearchType::Saved(_));
    }

    #[test]
    fn searching_for_times_should_work() {
        let query_result = parse_search_query("from past month");

        if let SearchType::Saved(query) = query_result {
            assert_eq!(query.tags.len(), 0);
            assert_eq!(query.negated_tags.len(), 0);
            assert_eq!(query.date_constraints.intervals.len(), 1);
        }
        else {
            panic!("Expected a Saved query, got something else");
        }
    }

    #[test]
    fn full_search_querys_should_work() {
        let query_result = parse_search_query("of things and not stuff; from past day");

        if let SearchType::Saved(search_query) = query_result {
            assert_eq!(search_query.tags, mapvec!(String::from: "things"));
            assert_eq!(search_query.negated_tags, mapvec!(String::from: "stuff"));

            println!("{:?}", search_query.date_constraints);
            assert!(search_query.date_constraints.intervals[0].contains(
                    &NaiveDateTime::from_timestamp(
                        (Utc::now() - ::chrono::Duration::hours(1)).timestamp(),
                        0
                    )
                )
            );
        }
        else {
            panic!("Search was not a SavedSearch");
        }
    }
}

#[cfg(test)]
mod private_query_tests {
    use super::*;

    #[test]
    fn tag_from_list_string_tests() {
        //Simple tags, no whitespaces
        assert_eq!(get_tags_from_list_string("some,thing,yolo"),
                  mapvec!(Cow::from: "some", "thing", "yolo"));

        //Whitespace
        assert_eq!(get_tags_from_list_string("not some,  thing   , not yo lo "),
                  mapvec!(Cow::from: "not some", "thing", "not yo lo"));
    }

    #[test]
    fn tag_negation_tests() {
        assert_eq!(separate_negated_tags(&mapvec!(Cow::from: "yolo", "not swag")),
                (mapvec!(String::from: "yolo"), mapvec!(String::from: "swag")));
    }
}
