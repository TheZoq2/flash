
use std::string::String;

use dotenv::dotenv;
use std::env;

use iron::typemap::Key;

use std::path::PathBuf;

#[derive(RustcEncodable, RustcDecodable)]
pub struct Settings
{
    file_storage_path: String,
    port: u32,
    file_read_path: PathBuf
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

        let file_read_path = {
            let as_str = env::var("FILE_READ_PATH")
                .expect("FILE_READ_PATH must be set, is .env missing?");

            PathBuf::from(as_str)
        };

        Settings
        {
            file_storage_path,
            port,
            file_read_path
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

    pub fn get_file_read_path(&self) -> PathBuf
    {
        self.file_read_path.clone()
    }
}

impl Key for Settings { type Value = Settings; }
