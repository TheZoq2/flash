extern crate iron;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use iron::*;
use persistent::{Write};

use file_util::sanitize_tag_names;

use file_database::FileDatabase;


pub fn handle_album_list_request(request: &mut Request) -> IronResult<Response>
{
    //Get the important information from the request.
    let tag_string = match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get("tags")
            {
                //The request contains a vec each occurence of the variable
                Some(val) => val.first().unwrap().clone(), 
                None => {
                    println!("Failed to save, tag list not included in the string");

                    //This is a lie. TODO: Update response
                    return Ok(Response::with(iron::status::NotFound));
                }
            }
        },
        Err(e) => {
            println!("Failed to get GET variable: {:?}", e); 

            //This is a lie. TODO: Update response
            return Ok(Response::with(iron::status::NotFound));
        }
    };

    //Decoding the json list
    let tags = match json::decode::<Vec<String>>(&tag_string){
        Ok(result) => sanitize_tag_names(&result).unwrap(),
        Err(e) => {
            println!("Failed to decode tag list. Error: {}", e);
            return Ok(Response::with(iron::status::NotFound));
        }
    };

    //Get the database and search through it for the tags
    //Store the file in the database
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let db = mutex.lock().unwrap();

    //let filenames = db_container.get_db().get_file_paths_with_tags(tags);
    let files = db.get_files_with_tags(tags);

    Ok(Response::with((status::Ok, json::encode(&files).unwrap())))
}


pub fn handle_album_image_request(request: &mut Request) -> IronResult<Response> 
{
    let id_string = match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get("id")
            {
                Some(val) => val.first().unwrap().clone(),
                None => {
                    println!("Failed to get file, no such tag");
                    return Ok(Response::with(iron::status::NotFound));
                }
            }
        },
        Err(e) =>
        {
            println!("Failed to get GET variable: {:?}", e); 
            return Ok(Response::with(iron::status::NotFound));//This is a lie. TODO: Update response
        }
    };

    let id = match id_string.parse::<i32>()
    {
        Ok(val) => val,
        Err(e) => {
            println!("Failed to decode image_request. ID: {} is not an integer. {}", id_string, e); 
            return Ok(Response::with(iron::status::NotFound));//This is a lie. TODO: Update response
        }
    };

    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let db = mutex.lock().unwrap();

    let file = db.get_file_with_id(id);

    Ok(Response::with((status::Ok, json::encode(&file).unwrap())))
}
