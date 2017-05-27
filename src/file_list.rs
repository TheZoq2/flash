use std::path::PathBuf;

use iron::typemap::Key;
use std::option::Option;

use file_util::{get_files_in_dir};


/**
  The location of a file stored in a file list.
*/
#[derive(Clone)]
enum FileLocation
{
    ///Not yet stored in the database.
    Unsaved(PathBuf),
    ///Stored in the database with the specified ID
    Database(i32)
}

/**
  Original source of creation of a FileList.
*/
#[derive(Clone)]
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

    pub fn add(&mut self, list: FileList)
    {
        self.lists.push(list)
    }
}

impl Key for FileListList { type Value = FileListList; }



