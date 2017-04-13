use file_database::FileEntry;

/**
  Returns true if the specified file has all of the specified tags
*/
fn has_all_tags(file: &FileEntry, tags: String) -> bool
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
fn has_some_tags(file: &FileEntry, tags: String) -> bool
{
    for tag in tags
    {
        if has_tag()
    }
}

