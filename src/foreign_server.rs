use error::{Result, ErrorKind, ResultExt};
use changelog::{SyncPoint, Change};
use chrono::NaiveDateTime;

use itertools::Itertools;

use serde_json;
use serde;

use futures::{Future, Stream, future};
use hyper::{Client, StatusCode};
use tokio_core::reactor::Core;

use std::str::from_utf8;

/**
  Struct of information about a file which can be requested from a `ForeginServer`
*/
#[derive(Clone, Serialize, Deserialize)]
pub struct FileDetails {
    pub extension: String,
    pub timestamp: NaiveDateTime
}

impl FileDetails {
    pub fn new(extension: String, timestamp: NaiveDateTime) -> Self {
        Self { extension, timestamp }
    }
}

/**
  Trait for communicating with another flash server
*/
pub trait ForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>;
    fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>>;
    fn get_file_details(&self, id: i32) -> Result<FileDetails>;
    fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()>;
    fn get_file(&self, id: i32) -> Result<Vec<u8>>;
    fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>>;
}



////////////////////////////////////////////////////////////////////////////////
//                      Http implementation
////////////////////////////////////////////////////////////////////////////////

const FOREIGN_SCHEME: &'static str = "http";

fn construct_url(scheme: &str, dns: &str, path: &[String], query: &[(String, String)]) -> String {
    let mut result = String::new();
    result += scheme;
    result += "://";
    result += dns;
    result += "/";

    let path_str = path.iter()
        .map(|s| s.clone())
        .intersperse("/".to_string())
        .collect::<String>();

    result += &path_str;

    result += "?";
    let query_str = query.iter()
        .map(|&(ref var, ref val)| var.clone() + "=" + &val)
        .intersperse("&".to_string())
        .collect::<String>();

    result += &query_str;

    result
}
/**
  Sends a request to the foreign server and parses the result as json for `T`
*/
fn send_request<'a, T: serde::de::DeserializeOwned>(full_url: String) -> Result<T> {
    let bytes = send_request_for_bytes(full_url)?;

    Ok(serde_json::from_str::<T>(from_utf8(&bytes)?)?)
}

fn send_request_for_bytes(full_url: String) -> Result<Vec<u8>> {
    // Parse the url into a hyper uri
    let uri = full_url.parse().chain_err(|| ErrorKind::ForeignHttpError(full_url.clone()))?;

    // Create a tokio core to exectue the request
    let mut core = Core::new()?;

    //Set up a hyper client client
    let client = Client::new(&core.handle());

    // Create the future
    let work = client.get(uri)
        .and_then(|res| {
            // Parse the chunks
            let status = res.status();
            (res.body().concat2(), future::ok(status))
        })
        .map(|(body, status)| -> Result<Vec<u8>> {
            match status {
                StatusCode::Ok => {
                    Ok(body.to_vec())
                }
                code => Err(ErrorKind::WrongHttpStatusCode(status).into())
            }
        });

    // Execute the future
    Ok(core.run(work)??)
}

struct HttpForeignServer {
    url: String,
}

impl HttpForeignServer {
    pub fn new(url: String) -> Self {
        Self {
            url,
        }
    }

    /**
      Returns a url on the form "self.url/sync/<action>?file_id=<id>"
    */
    fn get_file_sync_url(&self, file_id: i32, action: &str) -> String {
        let file_path = vec!(String::from("sync"), action.to_string());
        let query = vec!((String::from("file_id"), format!("{}", file_id)));
        construct_url("http", &self.url, &file_path, &query)
    }
}


impl ForeignServer for HttpForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>> {
        let syncpoint_path = vec!(String::from("sync"), String::from("syncpoints"));
        let url = construct_url(FOREIGN_SCHEME, &self.url, &syncpoint_path, &[]);

        Ok(send_request(url)?)
    }
    fn get_changes(&self, starting_syncpoint: &Option<SyncPoint>) -> Result<Vec<Change>> {
        let change_path = vec!(String::from("sync"), String::from("changes"));

        let query = match *starting_syncpoint {
            Some(SyncPoint{last_change}) => vec!((
                String::from("timestamp"),
                format!("{}", last_change.timestamp())
            )),
            None => vec!()
        };

        let url = construct_url(FOREIGN_SCHEME, &self.url, &change_path, &query);

        Ok(send_request(url)?)
    }
    fn get_file_details(&self, id: i32) -> Result<FileDetails> {
        let url = self.get_file_sync_url(id, "file_details");

        send_request::<FileDetails>(url)
    }
    fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()> {
        unimplemented!()
    }
    fn get_file(&self, id: i32) -> Result<Vec<u8>> {
        let url = self.get_file_sync_url(id, "file");

        Ok(send_request_for_bytes(url)?)
    }
    /**
      Gets the thumbnail of the file with the specified ID. If the content returned
      from the request is empty, `None` is returned
    */
    fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
        let url = self.get_file_sync_url(id, "thumbnail");

        let content = send_request_for_bytes(url)?;
        match content.len() {
            0 => Ok(None),
            _ => Ok(Some(content))
        }
    }
}


#[cfg(test)]
mod http_tests {
    use super::*;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Response {
        pub url: String
    }

    #[test]
    fn simple_request_test() {
        let response = send_request::<Response>("http://httpbin.org/get?test=true".to_string());

        let expected = Response{url: "http://httpbin.org/get?test=true".to_string()};
        assert_matches!(response, Ok(_));
        assert_eq!(response.unwrap(), expected);
    }

    #[test]
    fn statuscode_errors() {
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/404".to_string()),
            Err(_)
        );
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/500".to_string()),
            Err(_)
        );
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/200".to_string()),
            Ok(_)
        );
    }

    #[test]
    fn url_construction_test() {
        let url = construct_url(
            "https",
            "httpbin.org",
            &mapvec!(String::from: "path", "to"),
            &vec!(("var".to_string(), "val".to_string()), ("test".to_string(), "something".to_string()))
        );

        assert_eq!(url, "https://httpbin.org/path/to?var=val&test=something");
    }

    #[test]
    fn constructed_urls_are_valid() {
        let url = construct_url(
            "http",
            "httpbin.org",
            &vec!("get".to_string()),
            &vec!(("test".to_string(), "true".to_string()))
        );
        assert_eq!(url, "http://httpbin.org/get?test=true");
        let response = send_request::<Response>(url.clone());

        let expected = Response{url: url.to_string()};
        assert_matches!(response, Ok(_));
        assert_eq!(response.unwrap(), expected);
    }
}

