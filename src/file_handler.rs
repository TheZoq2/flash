use std::path::{PathBuf, Path};

use std::sync::mpsc::channel;

use file_database::{FileDatabase, File};
use file_util::{generate_thumbnail, get_file_timestamp};

use error::{ErrorKind, Error, Result};

use std::thread;

use std::fs;

pub enum ThumbnailStrategy<'a> {
    Generate,
    FromFile(&'a Path)
}

pub fn save_file(
        source_path: &Path,
        source_thumbnail: ThumbnailStrategy,
        fdb: &FileDatabase,
        id: i32,
        tags: &[String]
    )
    -> Result<(File, Receiver<FileSavingResult>)>
{
    //Get the folder where we want to place the stored file
    let destination_dir = PathBuf::from(fdb.get_file_save_path());

    let file_extension = match (*source_path).extension() {
        Some(val) => val,
        None => return Err(ErrorKind::NoFileExtension(source_path.clone().into()).into()),
    };

    let thumbnail_filename = format!("thumb_{}.jpg", id);

    let thumbnail_path = destination_dir.join(&PathBuf::from(thumbnail_filename));
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
            &thumbnail_filename.to_string(),
            tags,
            timestamp,
        )
    };

    // Spawn a thread to copy the files to their destinations
    let save_result_rx = {
        let original_path = source_path.clone();
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
}
