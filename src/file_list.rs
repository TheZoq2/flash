use std::path::PathBuf;

use iron::typemap::Key;
use std::option::Option;

use file_util::{get_files_in_dir};

use file_database;

/**
  The location of a file stored in a file list.
*/
#[derive(Clone, PartialEq, Debug)]
pub enum FileLocation
{
    ///Not yet stored in the database.
    Unsaved(PathBuf),
    ///Stored in the database as the specified file entry
    Database(file_database::File)
}

/**
  Original source of creation of a `FileList`.
*/
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FileListSource
{
    ///Result of a search query
    Search,
    ///Created from folder content
    Folder(PathBuf)
}

/**
  A list of files that are either from a file query or files stored in 
  a directry. Files can go from directory storage to database
*/
#[derive(Clone)]
pub struct FileList
{
    files: Vec<FileLocation>,
    source: FileListSource
}

impl FileList
{
    pub fn from_locations(files: Vec<FileLocation>, source: FileListSource) -> FileList
    {
        FileList {
            files,
            source
        }
    }

    pub fn from_directory(path: PathBuf) -> FileList
    {
        let file_paths = get_files_in_dir(&path);

        let files = file_paths
                .into_iter()
                .map(|path|{ FileLocation::Unsaved(path) })
                .collect();

        FileList {
            files: files,
            source: FileListSource::Folder(path)
        }
    }

    pub fn get(&self, index: usize) -> Option<&FileLocation>
    {
        self.files.get(index)
    }

    pub fn set(&mut self, index: usize, new_location: FileLocation) -> Result<(), ()>
    {
        if index < self.files.len()
        {
            self.files[index] = new_location;
            Ok(())
        }
        else
        {
            Err(())
        }
    }

    pub fn edit_entry(&self, index: usize, new_location: &FileLocation) -> FileList
    {
        let new_list = self.files
            .iter()
            .enumerate()
            .map(|(i, location): (usize, &FileLocation)| {
                    if i == index {
                        new_location.clone()
                    }
                    else {
                        location.clone()
                    }
                })
            .collect();

        FileList {
            files: new_list,
            .. self.clone()
        }
    }

    pub fn len(&self) -> usize
    {
        self.files.len()
    }

    pub fn get_files(&self) -> &Vec<FileLocation>
    {
        return &self.files;
    }
    pub fn get_source(&self) -> &FileListSource
    {
        return &self.source;
    }
}

/**
  A list of file lists
*/
pub struct FileListList
{
    lists: Vec<FileList>
}

impl FileListList
{
    pub fn new() -> FileListList
    {
        FileListList {
            lists: vec!()
        }
    }

    pub fn get(&self, index: usize) -> Option<&FileList>
    {
        self.lists.get(index)
    }

    pub fn add(&mut self, list: FileList) -> usize
    {
        self.lists.push(list);
        self.lists.len() - 1
    }

    pub fn get_id_with_source(&self, source: FileListSource) -> Option<usize>
    {
        //self.lists.iter().fold(None, |acc, elem| { acc || elem.source == source })
        for i in 0..self.lists.len()
        {
            if self.lists[i].source == source
            {
                return Some(i)
            }
        }
        None
    }

    /**
      Set the `FileLocation` in file `file_index` in list `list_id` to
      `file_entry`

      Does nothing if `file_index` or `list_id` are out of bounds
    */
    pub fn edit_file_list_entry(
            &mut self,
            list_id: usize,
            file_index: usize,
            file_entry: &FileLocation
        )
    {
        self.lists = self.lists
            .iter()
            .enumerate()
            .map(|enumerable| {
                    let (i, list) = enumerable;
                    if i == list_id {
                        list.edit_entry(file_index, file_entry)
                    }
                    else {
                        list.clone()
                    }
                })
            .collect()
    }
}

impl Key for FileListList { type Value = FileListList; }




#[cfg(test)]
mod file_list_tests
{
    use super::*;

    #[test]
    fn file_entry_update_test()
    {
        let mut fll = FileListList::new();

        let list1 = FileList::from_locations(
                vec!( FileLocation::Unsaved(PathBuf::from("test1"))
                    , FileLocation::Unsaved(PathBuf::from("test2"))
                    , FileLocation::Unsaved(PathBuf::from("test3"))
                    ),
                FileListSource::Search
            );
        let list2 = FileList::from_locations(
                vec!( FileLocation::Unsaved(PathBuf::from("test1"))
                    , FileLocation::Unsaved(PathBuf::from("test2"))
                    , FileLocation::Unsaved(PathBuf::from("test3"))
                    ),
                FileListSource::Search
            );

        fll.add(list1);
        fll.add(list2);

        fll.edit_file_list_entry(0,0, &FileLocation::Unsaved(PathBuf::from("changed")));

        assert_eq!(fll.get(0).unwrap().get(0).unwrap(), &FileLocation::Unsaved(PathBuf::from("changed")));
        assert_eq!(fll.get(0).unwrap().get(1).unwrap(), &FileLocation::Unsaved(PathBuf::from("test2")));
        assert_eq!(fll.get(0).unwrap().get(2).unwrap(), &FileLocation::Unsaved(PathBuf::from("test3")));

        assert_eq!(fll.get(1).unwrap().get(0).unwrap(), &FileLocation::Unsaved(PathBuf::from("test1")));
        assert_eq!(fll.get(1).unwrap().get(1).unwrap(), &FileLocation::Unsaved(PathBuf::from("test2")));
        assert_eq!(fll.get(1).unwrap().get(2).unwrap(), &FileLocation::Unsaved(PathBuf::from("test3")));
    }
}


