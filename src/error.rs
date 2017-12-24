error_chain! {
    links {
        Exif(::exiftool::Error, ::exiftool::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        SerdeJson(::serde_json::Error);
        Utf8(::std::string::FromUtf8Error);
    }

    errors {
        PersistentFileListLoadError {
            description("persistent file list read failed")
            display("Failed to read persistent file list")
        }
    }
}
