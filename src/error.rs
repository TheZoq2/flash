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
        DieselConnection(::diesel::ConnectionError);
        ImageError(::image::ImageError);
        Utf8(::std::string::FromUtf8Error);
        StrUtf8(::std::str::Utf8Error);
        Reqwest(::reqwest::Error);
        SystemTime(::std::time::SystemTimeError);
    }

    errors {
        #[cfg(test)]
        Dummy {
            description("Dummy error")
            display("Dummy error")
        }
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
        FileDatabaseRemovalFailed(id: i32) {
            description("The file could not be removed from the database")
            display("File {} could not be removed from database", id)
        }
        FileRemovalFailed(filename: String) {
            description("The file could not be removed")
            display("File {} could not be removed", filename)
        }

        ThumbnailGenerationFailed {
            description("Thumbnail generation failed")
            display("Failed to generate thumbnail")
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
        ChangeIDCollision(change_id: i32) {
            description("An ID collision occured when insertin change")
            display(
                "An ID collision occured when inserting a change with id id {}",
                change_id
            )
        }

        // Foreign server errors
        ForeignHttpError(url: String) {
            description("Failed to contact foreign server")
            display("Foreign server communication failed. Url: {}", url)
        }

        WrongHttpStatusCode(code: ::reqwest::StatusCode, body: String) {
            description("Got an unexpected HTTP statuscode")
            display("HTTP request returned status {}. Response: {}", code.as_u16(), body)
        }

        NoSuchJobId(id: usize) {
            description("No such job ID")
            display("No job with id {}", id)
        }
    }
}

impl ErrorKind {
    fn iron_status(&self) -> status::Status {
        match *self {
            ErrorKind::NoSuchVariable(_) |
            ErrorKind::InvalidVariableType(_, _) |
            ErrorKind::NoUrlEncodedQuery => status::Status::BadRequest,
            ErrorKind::UnknownAction(_) |
            ErrorKind::NoSuchList(_) |
            ErrorKind::NoSuchFileInList(_, _) |
            ErrorKind::NoSuchFileInDatabase(_) => status::Status::NotFound,
            _ => status::Status::InternalServerError
        }
    }
}


impl convert::From<Error> for IronError {
    fn from(source: Error) -> IronError {
        let message = format!("{:#?}\n", source);

        let status = source.iron_status();
        IronError {
            error: Box::new(source),
            response: Response::with((status, message)),
        }
    }
}

