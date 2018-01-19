use std::path::PathBuf;

use iron::{IronError, status, Response};
use std::convert;

error_chain! {
    links {
        Exif(::exiftool::Error, ::exiftool::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        SerdeJson(::serde_json::Error);
        Diesel(::diesel::result::Error);
        ImageError(::image::ImageError);
        Utf8(::std::string::FromUtf8Error);
    }

    errors {
        PersistentFileListLoadError {
            description("persistent file list read failed")
            display("Failed to read persistent file list")
        }

        NoFileExtension(path: PathBuf) {
            description("The specified path does not have an extension")
            display("Path {:?} does not have an extension", path)
        }

        // Errors relating to url variable parsing 
        NoSuchVariable(name: String) {
            description("Missing url variable")
            display("No variable named {}", name)
        }
        InvalidVariableType(name: String, t: String) {
            description("Wrong url variable type")
            display("Variable {} exists but is not {}", name, t)
        }
        NoUrlEncodedQuery {
            description("No url encoded query")
            display("The given URL contains no url encoded query")
        }
        UnknownAction(name: String) {
            description("The specified action was not understood")
            display("Unrecognised action {}", name)
        }


        // Intermediate errors
        ByteSourceExpansionFailed {
            description("An error occured when expanding the byte source")
            display("Byte source expansion failed")
        }

        // File handling errors
        FileRemovalFailed(id: i32) {
            description("The file could not be removed")
            display("File {} could not be removed", id)
        }


        // Errors specific to file requests
        NoSuchList(id: usize) {
            description("Unknown file list")
            display("no file list with id {}", id)
        }
        NoSuchFileInList(list_id: usize, file_id: usize) {
            description("Unknown file in list")
            display("No file with id {} in list {}", file_id, list_id)
        }

        // Database errors
        NoSuchFileInDatabase(file_id: i32) {
            description("The database did not contain a file with the specified id")
            display("The database did not contain a file with id {}", file_id)
        }
    }
}


impl convert::From<Error> for IronError {
    fn from(source: Error) -> IronError {
        let message = format!("{}", source);

        IronError {
            error: Box::new(source),
            // TODO: Correct HTTP error codes
            response: Response::with((status::NotFound, message)),
        }
    }
}
