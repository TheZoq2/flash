use error::{Result, ErrorKind};
use file_database::FileDatabase;
use settings::Settings;
use persistent::{Read};

use serde::{Serialize};
use serde::de::DeserializeOwned;
use serde_json;

use iron::prelude::*;

use urlencoded::UrlEncodedQuery;

/**
  Fetches a single GET variable of the specified `name` from the request.
  Errors if there are no GET variables or the specified variable could not be found
*/
pub fn get_get_variable(request: &mut Request, name: &str) -> Result<String> {
    match request.get_ref::<UrlEncodedQuery>() {
        Ok(hash_map) => {
            match hash_map.get(name) {
                Some(val) => Ok(val.first().unwrap().clone()),
                None => Err(ErrorKind::NoSuchVariable(name.to_owned()).into()),
            }
        }
        _ => Err(ErrorKind::NoUrlEncodedQuery.into()),
    }
}

/**
  Fetches a single number from the GET variables of the requests.
*/
pub fn get_get_i64(request: &mut Request, name: &str) -> Result<i64> {
    let string = get_get_variable(request, name)?;
    match string.parse::<i64>() {
        Ok(val) => Ok(val),
        Err(_) => {
            bail!(ErrorKind::InvalidVariableType("index".into(), "i64".into()))
        }
    }
}

/**
  Fetches a single number from the GET variables of the requests.
*/
pub fn get_get_usize(request: &mut Request, name: &str) -> Result<usize> {
    let string = get_get_variable(request, name)?;
    match string.parse::<usize>() {
        Ok(val) => Ok(val),
        Err(_) => {
            bail!(ErrorKind::InvalidVariableType("index".into(), "usize".into()))
        }
    }
}



/**
  runs serde_json::to_string and converts the result to error::Result instead
  of `serde_json::Result`
*/
pub fn to_json_with_result<T: Serialize>(data: T) -> Result<String> {
    Ok(serde_json::to_string(&data)?)
}

/**
  Runs serde_json::from_str and converts the result to error::Result instead of
  `serde_json::Result`
*/
pub fn from_json_with_result<'a, T: DeserializeOwned>(data: &str) -> Result<T> {
    Ok(serde_json::from_str(data)?)
}



/**
  Reads the settings variable from a request and sets up a database connection
*/
pub fn setup_db_connection(request: &mut Request) -> Result<FileDatabase>{
    let settings = request.get::<Read<Settings>>().unwrap();

    FileDatabase::new(&settings.database_url, settings.get_file_storage_path())
}
