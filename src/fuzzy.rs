use std::vec::Vec;

/**
  Runs a fuzzy search on a list of strings

  Returns the haystack ordered by match score
 */
fn fuzzy_search(needle: String, haystack: Vec<String>) -> Vec<String>
{
    for target in haystack
    {

    }
}


fn get_fuzzy_score(needle: String, target: String) -> i32
{
    let matched_chars = 0;

    for needle_char in needle
    {
        for target_char in target
        {
            if needle_char == target_char
            {
                matched_chars += 1;
            }
            else
            {

            }
        }
    }
}

/*
https://blog.forrestthewoods.com/reverse-engineering-sublime-text-s-fuzzy-match-4cffeed33fdb#.5v8eley5x
 */
