use std::path::{PathBuf, Path};

use std::sync::mpsc::{channel, Receiver};

use file_database::{FileDatabase, File};

use error::{Result, Error, ErrorKind, ResultExt};


use std::thread;

use std::fs;
use std::io::prelude::*;

use changelog::ChangeCreationPolicy;

use byte_source::{ByteSource, write_byte_source_to_file, vec_from_byte_source};

use file_util::generate_thumbnail;

// TODO: Remove if unused
#[derive(Debug)]
pub struct FileSavingWorkerResults {
    pub file: Receiver<Result<()>>,
    pub thumbnail: Option<Receiver<Result<()>>>,
}


pub enum ThumbnailStrategy {
    None,
    Generate,
    FromByteSource(ByteSource)
}

pub fn save_file(
        source_content: ByteSource,
        thumbnail_strategy: ThumbnailStrategy,
        id: i32,
        tags: &[String],
        fdb: &FileDatabase,
        change_policy: ChangeCreationPolicy,
        file_extension: &str,
        file_timestamp: u64,
    )
    -> Result<(File, FileSavingWorkerResults)>
{
    //Get the folder where we want to place the stored file
    let destination_dir = PathBuf::from(fdb.get_file_save_path());

    //Copy the file to the destination
    //Get the name and path of the new file
    let filename = format!("{}.{}", id, file_extension);
    let new_file_path = destination_dir.join(PathBuf::from(filename.clone()));

    // Save the thumbnail to disk
    let (thumbnail_filename, thumbnail_worker_result) =
        if let ThumbnailStrategy::None = thumbnail_strategy {
            (None, None)
        }
        else {
            let thumbnail_filename = format!("thumb_{}.jpg", id);
            let thumbnail_path = destination_dir.join(PathBuf::from(thumbnail_filename.clone()));

            let thumbnail_worker_result = match thumbnail_strategy {
                ThumbnailStrategy::Generate => {
                    Some(generate_thumbnail(source_content.clone(), &thumbnail_path))
                },
                ThumbnailStrategy::FromByteSource(data) => {
                    write_byte_source_to_file(data, &thumbnail_path)
                        .chain_err(|| "Failed to write thumbnail to disk")?;
                    None
                },
                ThumbnailStrategy::None => panic!("Unreachable statement")
            };

            (Some(thumbnail_filename), thumbnail_worker_result)
        };

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
            let save_result = save_file_to_disk(&new_file_path, source_content);

            // We ignore any failures to send the file save result since
            // it most likely means that the caller of the save function
            // does not care about the result
            match tx.send(save_result) {
                _ => {}
            }
        });

        rx
    };

    Ok((
        saved_file,
        FileSavingWorkerResults{
            file: save_result_rx,
            thumbnail: thumbnail_worker_result
        }
    ))
}

fn save_file_to_disk(destination_path: &Path, content: ByteSource) -> Result<()>  {
    let mut file = fs::File::create(destination_path)?;

    Ok(file.write_all(&vec_from_byte_source(content)?)?)
}


/**
  Drops a file from the database and removes it from the file system.

  Creates a change if the `change_policy` says to do so
*/
pub fn remove_file(file_id: i32, fdb: &FileDatabase, change_policy: ChangeCreationPolicy) -> Result<()> {
    // Fetch the file details from the database
    let file = fdb.get_file_with_id_result(file_id)
        .chain_err(|| ErrorKind::FileDatabaseRemovalFailed(file_id))?;

    // Drop the file from the database
    fdb.drop_file(file_id, change_policy)?;

    let full_path = fdb.get_file_save_path().join(file.filename);

    fs::remove_file(full_path.clone())
        .chain_err(|| ErrorKind::FileRemovalFailed(full_path.to_string_lossy().into()))?;

    Ok(())
}





