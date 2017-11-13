use std::path::{PathBuf, Path};

use file_database::FileDatabase;

use error::{ErrorKind, Error, Result};

pub enum ThumbnailStrategy<'a> {
    Generate,
    FromFile(&'a Path)
}

pub fn save_file(
        source_path: &Path,
        source_thumbnail: ThumbnailStrategy,
        fdb: &FileDatabase,
        id: f32
    )
    -> Result<(file_database::File, Receiver<FileSavingResult>)>
{
    let file_extension = match (*original_path).extension() {
        Some(val) => val,
        None => return Err(ErrorKind::NoFileExtension(original_path.clone()).into()),
    };

    let thumbnail_filename = format!("thumb_{}.jpg", file_identifier);

    let thumbnail_path = destination_dir.join(&PathBuf::from(thumbnail_filename));
    match source_thumbnail {
        ThumbnailStrategy::Generate => {

            //Generate the thumbnail
            generate_thumbnail(original_path, &thumbnail_path, 300)?;
        }
        ThumbnailStrategy::FromFile(thumbnail_path) => {

        }
    }
}
