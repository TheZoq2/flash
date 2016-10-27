use std::path::PathBuf;

use iron::typemap::Key;
use std::option::Option;


#[derive(Clone)]
pub struct File
{
    pub path: PathBuf,
    pub saved_id: Option<usize>
}

#[derive(Clone)]
pub struct FileList
{
    files: Vec<File>,
    current_index: usize,
}

impl FileList
{
    pub fn new(file_paths: Vec<PathBuf>) -> FileList 
    {
        let mut files = vec!();
        for path in file_paths
        {
            files.push(File{path:path, saved_id: None})
        }
        
        FileList {
            files: files,
            current_index: 0,
        }
    }

    pub fn get_current_file(&self) -> Option<File>
    {
        if self.current_index < self.files.len()
        {
            return Some(self.files[self.current_index].clone());
        }

        None
    }
    pub fn get_current_file_save_id(&self) -> Option<usize>
    {
        match self.get_current_file()
        {
            Some(val) => val.saved_id,
            None => None
        }
    }

    /**
      Returns the file after the current file without incrementing the current index. This can
      be used to preload the images in order to prevent the small lag when loading new images.
     */
    pub fn peak_next_file(&self) -> Option<File> 
    {
        if self.current_index + 1 < self.files.len()
        {
            return Some(self.files[self.current_index + 1].clone());
        }
        None
    }
    /**
      Increments current index by one while making sure it doesn't go too far out of bounds
     */
    pub fn select_next_file(&mut self)
    {
        self.current_index += 1;

        if self.current_index > self.files.len()
        {
            self.current_index = self.files.len();
        }
    }
    /**
      Decrements current index by one while making sure it doesn't go too far out of bounds
     */
    pub fn select_prev_file(&mut self)
    {
        if self.current_index > 1
        {
            self.current_index -= 1;
        }
    }

    pub fn mark_current_file_as_saved(&mut self, db_id: usize)
    {
        if self.current_index < self.files.len()
        {
            self.files[self.current_index].saved_id = Some(db_id);
        }
    }
}

impl Key for FileList { type Value = FileList; }


