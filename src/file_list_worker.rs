use iron::typemap::Key;

use std::sync::mpsc;
use std::thread;

use std::path::PathBuf;


use persistent_file_list::{SaveableFileList, save_file_list_list};

pub enum Command {
    Save(Vec<SaveableFileList>),
}

pub struct Commander {
    sender: mpsc::Sender<Command>,
}

impl Commander {
    pub fn new(sender: mpsc::Sender<Command>) -> Self {
        Self { sender }
    }

    pub fn send(&self, command: Command) -> Result<(), mpsc::SendError<Command>> {
        self.sender.send(command)
    }
}

impl Key for Commander {
    type Value = Commander;
}

/**
  A worker thread for taking care of asyncronous changes to file lists
  */
pub fn start_worker(save_path: PathBuf) -> Commander {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        // Listen for new messages on the channel, exit the thread if
        // the sender has disconnected
        while let Ok(message) = receiver.recv() {
            // Handle the commands
            match message {
                Command::Save(list) => save_file_list_list(&list, &save_path).unwrap(),
            }
        }
    });

    Commander::new(sender)
}
