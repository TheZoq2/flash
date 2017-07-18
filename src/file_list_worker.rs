use iron::typemap::Key;

use std::sync::mpsc;
use std::path::PathBuf;

pub enum FileListListWorkerCommand
{
    Save(Vec<SaveableFileList>)
}

impl Key for FileListListWorkerCommand { type Value = FileListListWorkerCommand; }

/**
  A worker thread for taking care of asyncronous changes to file lists
*/
pub fn start_file_list_worker(save_path: PathBuf) -> mpsc::Sender<FileListListWorkerCommand>
{
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        // Listen for new messages on the channel, exit the thread if
        // the sender has disconnected
        while let Ok(message) = receiver.recv()
        {
            // Handle the commands
            match message
            {
                FileListListWorkerCommand::Save(list) =>
                    save_file_list_list(&list, &save_path).unwrap()
            }
        }
    });

    sender
}
