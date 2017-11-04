use error::{Result, ErrorKind};

use iron::*;

use urlencoded::UrlEncodedQuery;

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
