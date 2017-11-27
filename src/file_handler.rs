use std::path::{PathBuf, Path};

use std::sync::mpsc::{channel, Receiver};

use file_database::{FileDatabase, File};
use file_util::{generate_thumbnail, get_file_timestamp};

use error::{ErrorKind, Error, Result};

use chrono::NaiveDateTime;

use std::thread;

use std::fs;
use std::io;
use std::io::prelude::*;

use changelog::ChangeCreationPolicy;

// TODO: Remove if unused
#[derive(Debug)]
pub enum FileSavingResult {
    Success,
    Failure(io::Error),
}


pub enum ThumbnailStrategy<'a> {
    Generate,
    FromFile(&'a Path)
}

pub fn save_file(
        source_content: &[u8],
        source_thumbnail: Option<&[u8]>,
        id: i32,
        tags: &[String],
        fdb: &mut FileDatabase,
        create_change: bool,
        file_extension: &str,
        change_timestamp: NaiveDateTime
    )
    -> Result<(File, Receiver<FileSavingResult>)>
{
    //Get the folder where we want to place the stored file
    let destination_dir = PathBuf::from(fdb.get_file_save_path());

    let thumbnail_filename = format!("thumb_{}.jpg", id);

    let thumbnail_path = destination_dir.join(&PathBuf::from(thumbnail_filename.clone()));
    match source_thumbnail {
        ThumbnailStrategy::Generate => {

            //Generate the thumbnail
            generate_thumbnail(source_path, &thumbnail_path, 300)?;
        }
        ThumbnailStrategy::FromFile(thumbnail_path) => {
            unimplemented!()
        }
    }

    //Copy the file to the destination
    //Get the name and path of the new file
    let filename = format!("{}.{}", id, file_extension.to_string_lossy());
    let new_file_path = destination_dir.join(PathBuf::from(filename.clone()));

    let timestamp = get_file_timestamp(source_path);

    //Store the file in the database
    let saved_file = {
        fdb.add_new_file(
            id,
            &filename.to_owned(),
            Some(&thumbnail_filename.to_string()),
            tags,
            timestamp,
            ChangeCreationPolicy::Yes(change_timestamp),
        )
    };

    // Spawn a thread to copy the files to their destinations
    let save_result_rx = {
        let original_path = PathBuf::from(source_path.clone());
        let new_file_path = new_file_path.clone();

        let (tx, rx) = channel();

        thread::spawn(move || {
            let save_result = match fs::copy(original_path, new_file_path) {
                Ok(_) => FileSavingResult::Success,
                Err(e) => FileSavingResult::Failure(e),
            };

            // We ignore any failures to send the file save result since
            // it most likely means that the caller of the save function
            // does not care about the result
            match tx.send(save_result) {
                _ => {}
            }
        });

        rx
    };

    Ok((saved_file, save_result_rx))
}

fn save_file_to_disk(destination_path: &Path, content: &[u8]) -> ::std::io::Result<()> {
    let mut file = fs::File::create(destination_path)?;
    Ok(file.write_all(content)?)
}


pub fn remove_file(file_id: i32, fdb: &FileDatabase, create_change: bool) -> Result<()> {
    // Fetch the file details from the database
    if create_change {
        fdb.drop_file_without_creating_change(file_id)?;
    }
    else {
        unimplemented!()
    }

    // Remove the actual file in the file system
    unimplemented!();
}

