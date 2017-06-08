use iron::{IronError, status, Response};

use std::error::Error;
use std::fmt;
use std::convert;

use image;
use std::path::PathBuf;


#[derive(Debug)]
pub enum FileRequestError
{
    /// There is no `FileList` with the specified ID
    NoSuchList(usize),
    /// The `FileList` with `id` does not contain a file with `id`
    NoSuchFile(usize, usize),
    /// The request did not contain the speicfied variable
    NoSuchVariable(String),
    /// The request contained `variable` but it was not `type`
    // TODO: Add a box for the root cause of the error
    InvalidVariableType(String, String),
    /// The current request did not contain a `UrlEncodedQuery`
    NoUrlEncodedQuery,
    /// Error returned when a thumbnail was generated
    ThumbnailGenerationError(image::ImageError),
    /// Error thrown when a function expected a path with an extension but
    /// got a file without one
    NoFileExtension(PathBuf)
}

impl fmt::Display for FileRequestError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{:?}", *self)
    }
}

impl convert::From<image::ImageError> for FileRequestError
{
    fn from(source: image::ImageError) -> Self
    {
        FileRequestError::ThumbnailGenerationError(source)
    }
}

impl Error for FileRequestError
{
    fn description(&self) -> &str
    {
        match self
        {
            &FileRequestError::NoSuchList(_) =>
                "Unknown file list",
            &FileRequestError::NoSuchFile(_, _) =>
                "Unknown file",
            &FileRequestError::NoSuchVariable(_) =>
                "Missing url variable",
            &FileRequestError::InvalidVariableType(_, _) =>
                "Wrong url variable type",
            &FileRequestError::NoUrlEncodedQuery =>
                "No url parameters",
            &FileRequestError::ThumbnailGenerationError(_) =>
                "Failed to generate thumbnail",
            &FileRequestError::NoFileExtension(_) =>
                "The specified path does not have an extension"
        }
    }
}

impl convert::From<FileRequestError> for IronError {
    fn from(source: FileRequestError) -> IronError
    {
        let message = format!("{}", source);

        IronError
        {
            error: Box::new(source),
            response: Response::with((status::NotFound, message))
        }
    }
}





/**
  Convenience function for avoiding String::from for creating 
  `FileRequestError::InvalidVariableType` errors
*/
pub fn err_invalid_variable_type(var: &str, expected_type: &str) -> FileRequestError
{
    FileRequestError::InvalidVariableType(var.to_owned(), expected_type.to_owned())
}

