
use std::string::String;

#[derive(RustcEncodable, RustcDecodable)]
pub struct Settings
{
    file_storage_path: String,
}

impl Settings
{
    pub fn get_defaults() -> Settings
    {
        Settings
        {
            file_storage_path: "/home/frans/Pictures/flash".to_string(),
        }
    }

    pub fn get_file_storage_path(&self) -> String
    {
        return self.file_storage_path.clone();
    }
    pub fn get_database_save_path(&self) -> String
    {
        self.get_file_storage_path() + "/" + "database.json"
    }
}
