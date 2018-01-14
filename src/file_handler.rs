use std::path::{PathBuf, Path};

use std::sync::mpsc::{channel, Receiver};

use file_database::{FileDatabase, File};

use error::{Result, Error};


use std::thread;

use std::fs;
use std::io::prelude::*;

use std::sync::Arc;

use changelog::ChangeCreationPolicy;

use byte_source::ByteSource;

// TODO: Remove if unused
#[derive(Debug)]
pub enum FileSavingResult {
    Success,
    Failure(Error),
}


pub enum ThumbnailStrategy {
    None,
    Generate,
    FromByteSource(Arc<ByteSource>)
}

pub fn save_file(
        source_content: Arc<ByteSource>,
        thumbnail_strategy: ThumbnailStrategy,
        id: i32,
        tags: &[String],
        fdb: &FileDatabase,
        change_policy: ChangeCreationPolicy,
        file_extension: &str,
        file_timestamp: u64,
    )
    -> Result<(File, Receiver<FileSavingResult>)>
{
    //Get the folder where we want to place the stored file
    let destination_dir = PathBuf::from(fdb.get_file_save_path());

    //Copy the file to the destination
    //Get the name and path of the new file
    let filename = format!("{}.{}", id, file_extension);
    let new_file_path = destination_dir.join(PathBuf::from(filename.clone()));

    // Save the thumbnail to disk
    let thumbnail_filename = if let ThumbnailStrategy::None = thumbnail_strategy {
        None
    }
    else {
        let thumbnail_filename = format!("thumb_{}.jpg", id);
        let thumbnail_path = destination_dir.join(PathBuf::from(thumbnail_filename.clone()));

        Some(thumbnail_filename)
    };

    //let timestamp = get_file_timestamp(source_path);

    //Store the file in the database
    let saved_file = {
        fdb.add_new_file(
            id,
            &filename,
            thumbnail_filename.as_ref().map(|x| &**x),
            tags,
            file_timestamp,
            change_policy
        )
    };

    // Spawn a thread to copy the files to their destinations
    let save_result_rx = {
        let (tx, rx) = channel();

        thread::spawn(move || {
            let save_result = match save_file_to_disk(&new_file_path, source_content) {
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

fn save_file_to_disk(destination_path: &Path, content: Arc<ByteSource>) -> Result<()>  {
    let mut file = fs::File::create(destination_path)?;
    let mut bytes = vec!();

    while let Some(b) = content.next() {
        bytes.push(b?)
    }
    Ok(file.write_all(&bytes)?)
}


/**
  Drops a file from the database and removes it from the file system.

  Creates a change if the `change_policy` says to do so
*/
pub fn remove_file(file_id: i32, fdb: &FileDatabase, change_policy: ChangeCreationPolicy) -> Result<()> {
    // Fetch the file details from the database
    let file = fdb.get_file_with_id_result(file_id)?;

    // Drop the file from the database
    fdb.drop_file(file_id, change_policy)?;

    // Remove the actual file in the file system
    unimplemented!("Perform the change in the file system");

    Ok(())
}





