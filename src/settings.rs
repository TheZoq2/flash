
use std::string::String;

use dotenv::dotenv;
use std::env;

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

    pub fn from_env() -> Settings
    {
        dotenv().ok();

        let file_storage_path = env::var("FILE_STORAGE_PATH")
            .expect("FILE_STORAGE_PATH must be set, is .env missing?");

        Settings
        {
            file_storage_path
        }
    }

    pub fn get_file_storage_path(&self) -> String
    {
        self.file_storage_path.clone()
    }
}
