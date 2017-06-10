use file_request_error::FileRequestError;
use iron::*;

use urlencoded::UrlEncodedQuery;

pub fn get_get_variable(request: &mut Request, name: &str) -> Result<String, FileRequestError>
{
    match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get(name)
            {
                Some(val) => Ok(val.first().unwrap().clone()),
                None => Err(FileRequestError::NoSuchVariable(name.to_owned()))
            }
        },
        _ => Err(FileRequestError::NoUrlEncodedQuery)
    }
}
