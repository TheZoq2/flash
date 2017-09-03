error_chain!
{
    foreign_links
    {
        Io(::std::io::Error);
        SerdeJson(::serde_json::Error);
    }

    errors
    {
        PersistentFileListLoadError {
            description("persistent file list read failed")
            display("Failed to read persistent file list")
        }
    }
}
