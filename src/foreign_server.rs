use error::{Result, ErrorKind, ResultExt};
use changelog::{SyncPoint, Change};
use chrono::NaiveDateTime;

use serde_json;
use serde;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

use std::str::from_utf8;

/**
  Struct of information about a file which can be requested from a `ForeginServer`
*/
#[derive(Clone)]
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
      Sends a request to the foreign server and parses the result as `T`
    */
    fn send_request<'a, T: serde::de::DeserializeOwned>(&self, request: &str) -> Result<T> {
        // Combine the url and request
        let full_url = self.url.clone() + request;
        let uri = full_url.parse().chain_err(|| ErrorKind::ForeignHttpError(full_url.clone()))?;

        // Create a tokio core to exectue the request
        let mut core = Core::new()?;

        //Set up a hyper client client
        let client = Client::new(&core.handle());

        // Create the future
        let work = client.get(uri)
            .and_then(|res| {
                // Parse the chunks
                res.body().concat2()
            })
            .map(|body| -> Result<T> {
                Ok(serde_json::from_str::<T>(from_utf8(body.as_ref())?)?)
            });

        // Execute the future
        core.run(work)?
    }
}


impl ForeignServer for HttpForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>> {
        unimplemented!()
    }
    fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>> {
        unimplemented!()
    }
    fn get_file_details(&self, id: i32) -> Result<FileDetails> {
        unimplemented!()
    }
    fn send_changes(&self, changes: &[Change], new_syncpoint: &SyncPoint) -> Result<()> {
        unimplemented!()
    }
    fn get_file(&self, id: i32) -> Result<Vec<u8>> {
        unimplemented!()
    }
    fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
        unimplemented!()
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;

    #[test]
    fn simple_request_test() {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Response {
            pub url: String
        }

        let foreign_server = HttpForeignServer::new("http://httpbin.org".to_string());
        let response = foreign_server.send_request::<Response>("/get?test=true");

        let expected = Response{url: "http://httpbin.org/get?test=true".to_string()};
        assert_matches!(response, Ok(expected));
    }
}

