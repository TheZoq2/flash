use error::{Result, ErrorKind, ResultExt};
use changelog::{SyncPoint, Change};
use chrono::NaiveDateTime;

use itertools::Itertools;

use serde_json;
use serde;

use futures::{Future, Stream, future};
use hyper::{
    self,
    Client,
    StatusCode,
    Request,
    Method,
    header,
};
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

impl<'a> From<&'a ::file_database::File> for FileDetails {
    fn from(file: &'a ::file_database::File) -> Self {
        let extension = ::std::path::PathBuf::from(file.filename.clone())
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or("".to_string());

        FileDetails {
            extension,
            timestamp: file.creation_date
        }
    }
}

/**
  Data about changes that should be applied on a foreign server

  `syncpoint` is the new syncpoint that should be created
*/
#[derive(Serialize, Deserialize)]
pub struct ChangeData {
    pub changes: Vec<Change>,
    pub syncpoint: SyncPoint,
    pub removed_files: Vec<i32>
}

/**
  Trait for communicating with another flash server
*/
pub trait ForeignServer {
    fn get_syncpoints(&self) -> Result<Vec<SyncPoint>>;
    fn get_changes(&self, starting_timestamp: &Option<SyncPoint>) -> Result<Vec<Change>>;
    fn get_file_details(&self, id: i32) -> Result<FileDetails>;
    fn send_changes(&mut self, change: &ChangeData, own_port: u16) -> Result<usize>;
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
fn send_request<'a, T: serde::de::DeserializeOwned>(full_url: &str, body: &str) -> Result<T> {
    let bytes = send_request_for_bytes(full_url, body)?;

    Ok(serde_json::from_str::<T>(from_utf8(&bytes)?)?)
}

fn send_request_for_bytes(full_url: &str, body: &str) -> Result<Vec<u8>> {
    // Parse the url into a hyper uri
    let uri = full_url.parse().chain_err(
        || ErrorKind::ForeignHttpError(full_url.clone().to_string())
    )?;

    // Create a tokio core to exectue the request
    let mut core = Core::new()?;

    //Set up a hyper client client
    let client = Client::new(&core.handle());

    let mut request: Request<hyper::Body> = Request::new(Method::Get, uri);
    request.set_body(body.to_string());
    request.headers_mut().set(header::ContentLength(body.len() as u64));

    // Create the future
    let work = client.request(request)
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
                code => {
                    let body = String::from_utf8(body.to_vec())
                        .unwrap_or("Invalid UTF8".to_string());
                    Err(ErrorKind::WrongHttpStatusCode(code, body.to_string()).into())
                }
            }
        });

    // Execute the future
    Ok(core.run(work)??)
}

pub struct HttpForeignServer {
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

        Ok(send_request(&url, "")?)
    }
    fn get_changes(&self, starting_syncpoint: &Option<SyncPoint>) -> Result<Vec<Change>> {
        let change_path = vec!(String::from("sync"), String::from("changes"));

        let starting_timestamp = match *starting_syncpoint {
            Some(SyncPoint{last_change}) => last_change.timestamp(),
            None => 0
        };

        let query = vec!(
            (String::from("starting_timestamp"), format!("{}", starting_timestamp))
        );

        let url = construct_url(FOREIGN_SCHEME, &self.url, &change_path, &query);

        Ok(send_request(&url, "")?)
    }
    fn get_file_details(&self, id: i32) -> Result<FileDetails> {
        let url = self.get_file_sync_url(id, "file_details");

        send_request::<FileDetails>(&url, "")
    }
    fn send_changes(&mut self, changes: &ChangeData, own_port: u16) -> Result<usize> {
        let path = vec!(String::from("sync"), String::from("apply_changes"));
        let query = vec!((String::from("port"), format!("{}", own_port)));
        let url = construct_url(FOREIGN_SCHEME, &self.url, &path, &query);

        let body = serde_json::to_string(changes)?;

        Ok(send_request::<usize>(&url, &body)?)
    }
    fn get_file(&self, id: i32) -> Result<Vec<u8>> {
        let url = self.get_file_sync_url(id, "file");

        Ok(send_request_for_bytes(&url, "")?)
    }
    /**
      Gets the thumbnail of the file with the specified ID. If the content returned
      from the request is empty, `None` is returned
    */
    fn get_thumbnail(&self, id: i32) -> Result<Option<Vec<u8>>> {
        let url = self.get_file_sync_url(id, "thumbnail");

        let content = send_request_for_bytes(&url, "")?;
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
        let response = send_request::<Response>("http://httpbin.org/get?test=true", "");

        let expected = Response{url: "http://httpbin.org/get?test=true".to_string()};
        assert_matches!(response, Ok(_));
        assert_eq!(response.unwrap(), expected);
    }

    #[test]
    fn statuscode_errors() {
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/404", ""),
            Err(_)
        );
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/500", ""),
            Err(_)
        );
        assert_matches!(
            send_request_for_bytes("http://httpbin.org/status/200", ""),
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
        let response = send_request::<Response>(&url, "");

        let expected = Response{url: url.to_string()};
        assert_matches!(response, Ok(_));
        assert_eq!(response.unwrap(), expected);
    }
}


#[cfg(test)]
mod sync_integration {
    use super::*;

    use file_list_response::ListResponse;

    use std::time::Duration;
    use std::thread;
    use sync_progress::SyncStatus;
    use sync_progress::SyncUpdate;

    /**
      Runs the foreign server by launching the test script and kills it when
      it goes out of scope. This ensures that the server does not keep running
      after failed assertions
    */
    struct ForeignServerRunner {
        pid: String
    }
    impl ForeignServerRunner {
        pub fn run(port: &str) -> Option<Self> {
            #[derive(Deserialize)]
            struct TestStarterOutput {
                pub pid: String
            }

            // Start the foreign server
            let mut command = ::std::process::Command::new("test/run_sync_test_server.sh");

            let command = command.arg("-p").arg(port);

            // Read the output from the startup script
            let output = command.output().expect("Failed to read test server starter output");

            // Ensure that the test server starts correctly
            if !output.status.success() {
                panic!("Test server startup failed. Output: {:#?}", output);
            };

            // Get the pid of the test server to kill it later
            let test_script_output = String::from_utf8(output.stdout)
                .expect("Output was not valid utf-8");

            let as_object: Option<TestStarterOutput>= serde_json::from_str(&test_script_output)
                .expect(&format!("Test script outputed {} which is not valid json", test_script_output));

            as_object.map(|output| Self{pid: output.pid})
        }
    }
    impl ::std::ops::Drop for ForeignServerRunner {
        fn drop(&mut self) {
            // Kill the test server as it is no longer needed
            ::std::process::Command::new("kill")
                .arg(self.pid.clone())
                .output()
                .expect("Failed to kill test server");

            println!("dropping foreignserverrunner");
        }
    }

    #[test]
    fn sync_works() {
        let _process1 = ForeignServerRunner::run("3001");
        let _process1 = ForeignServerRunner::run("3002");

        let url1 = "localhost:3001";
        let url2 = "localhost:3002";

        // Save some files in each database
        let listid1 = create_file_list_on_foreign(url1, "/");
        let listid2 = create_file_list_on_foreign(url2, "/");
        save_file(url1, listid1, 0, vec!("from1".into()));
        save_file(url1, listid1, 1, vec!("from1".into()));
        save_file(url2, listid2, 2, vec!("from2".into()));
        save_file(url2, listid2, 3, vec!("from2".into()));


        // Run sync
        let job_id = sync_with_foreign(url1, url2);

        // Give the servers a second to finnish
        let mut iterations = 0;
        // We have to wait for the servers to add the new job to the
        // list of jobs. Perhaps this should be done in a more robust way
        thread::sleep(Duration::from_millis(100));
        loop {
            let (local_update, foreign_update) = check_job_status(url1, job_id, url2);
            if let (&SyncUpdate::Done, &Some(SyncUpdate::Done))
                = (&local_update, &foreign_update) {
                break;
            }

            if iterations > 10 {
                assert!(
                    false,
                    "Still waiting on sync jobs ({:?} {:?})",
                    local_update,
                    foreign_update
                )
            }
            iterations += 1;
            thread::sleep(Duration::from_millis(100));
        }

        // Ensure that all files have been synced
        assert_eq!(file_list_request(url1, "of+from2").length, 2);
        assert_eq!(file_list_request(url2, "of+from1").length, 2);
    }

    fn save_file(url: &str, list_id: usize, file_index: u32, tags: Vec<String>) {
        let save_url = construct_url(
                "http",
                url,
                &vec!("list".into()),
                &vec!(
                    ("action".into(), "save".into()),
                    ("list_id".into(), format!("{}", list_id)),
                    ("index".into(), format!("{}", file_index)),
                    ("tags".into(), serde_json::to_string(&tags).unwrap())
                )
            );

        send_request_for_bytes(&save_url, "").expect("failed to save image");
    }

    fn create_file_list_on_foreign(url: &str, path: &str) -> usize {
        // Create a new file list
        let list_url = construct_url(
            "http",
            url,
            &vec!("search".into()),
            &vec!(("query".into(), path.into()))
        );

        send_request::<ListResponse>(&list_url, "")
            .unwrap()
            .id
    }

    fn file_list_request(url: &str, query: &str) -> ListResponse {
        // Create a new file list
        let list_url = construct_url(
            "http",
            url,
            &vec!("search".into()),
            &vec!(("query".into(), query.into()))
        );

        println!("list_url: {}", list_url);

        send_request::<ListResponse>(&list_url, "")
            .unwrap()
    }

    fn sync_with_foreign(url: &str, foreign_url: &str) -> usize {
        let request_url = construct_url(
                "http",
                url,
                &vec!("sync".into(), "sync".into()),
                &vec!(("foreign_url".into(), foreign_url.into()))
            );

        println!("{}", request_url);

        send_request::<usize>(&request_url, "").expect("Sync failed")
    }

    fn check_job_status(initiating_url: &str, initiating_job_id: usize, foreign_url: &str)
        -> (SyncUpdate, Option<SyncUpdate>)
    {
        let request_url = construct_url(
                "http",
                initiating_url,
                &vec!("sync".into(), "progress".into()),
                &vec!(("job_id".into(), format!("{}", initiating_job_id).into()))
            );

        println!("Foreign job status url {}", request_url);

        let initiating_status = send_request::<SyncStatus>(&request_url, "")
            .expect("Job status query failed");

        let foreign_status = initiating_status.foreign_job_id.map(|job_id| {
            let request = construct_url(
                "http",
                foreign_url,
                &vec!("sync".into(), "progress".into()),
                &vec!(("job_id".into(), format!("{}", job_id).into()))
            );

            send_request::<SyncStatus>(&request, "")
                .expect("Foreign job status query failed")
                .last_update
        });

        (initiating_status.last_update, foreign_status)
    }
}
