
use std::string::String;

use dotenv::dotenv;
use std::env;

#[derive(RustcEncodable, RustcDecodable)]
pub struct Settings
{
    file_storage_path: String,
    port: u32
}

impl Settings
{
    pub fn from_env() -> Settings
    {
        dotenv().ok();

        let file_storage_path = env::var("FILE_STORAGE_PATH")
            .expect("FILE_STORAGE_PATH must be set, is .env missing?");

        let port = env::var("FLASH_PORT")
            .unwrap_or("3000".to_owned())
            .parse::<u32>()
            .expect("FLASH_PORT must be a positive integer");

        Settings
        {
            file_storage_path,
            port
        }
    }

    pub fn get_file_storage_path(&self) -> String
    {
        self.file_storage_path.clone()
    }

    pub fn get_port(&self) -> u32
    {
        self.port
    }
}
