use std::path::PathBuf;

use iron::typemap::Key;
use std::option::Option;


#[derive(Clone)]
enum FileLocation
{
    Unsaved(PathBuf),
    Database(i32)
}

#[derive(Clone)]
pub struct FileList
{
    files: Vec<FileLocation>,
}

impl FileList
{
    pub fn from_file_paths(file_paths: Vec<PathBuf>) -> FileList 
    {
        let mut files = vec!();
        for path in file_paths
        {
            files.push(FileLocation::Unsaved(path))
        }

        FileList {
            files: files,
        }
    }

    pub fn get(&self, index: usize) -> Option<&FileLocation>
    {
        self.files.get(index)
    }
}

impl Key for FileList { type Value = FileList; }


