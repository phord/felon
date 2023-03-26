// Reader of unseekable text streams
// For a stream we have to store old lines in RAM to be able to seek around.

// Options:
//  We can load the file lazily, on-demand.
//  We can spool the data to a temp file and then mmap it.

use std::path::PathBuf;
use std::fs::File;
use std::fmt;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::files::LogFileTrait;

enum Source {
    Stdin,
    File(BufReader<File>),
}

pub struct TextLogStream {
    // pub file_path: PathBuf,
    stream: Rc<RefCell<StreamBuffer>>,
}

impl fmt::Debug for TextLogStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextLogStream")
         .field("bytes", &self.len())
         .finish()
    }
}

impl LogFileTrait for TextLogStream {
    fn len(&self) -> usize {
        self.stream.borrow().len()
    }

    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        todo!("Let read return a string");
        self.stream.borrow_mut().read(offset, len)
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        // We only ever chunk-read to append
        assert!(target >= self.len());
        (target, target + 1)
    }
}

struct StreamBuffer {
    buffer: String,
    src: Source,
}

impl StreamBuffer {

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn blocking_read(&mut self) -> bool {
        let mut buf = String::new();
        match &mut self.src {
            Source::Stdin =>
                // BLOCKING READ; returns 0 bytes when eof()
                io::stdin().lock().read_line(&mut buf).unwrap(),
            Source::File(file) => file.read_line(&mut buf).unwrap(),
        };
        let eof = buf.is_empty();
        self.buffer += &buf;
        eof
    }

    fn read(&mut self, offset: usize, len: usize) -> Option<&[u8]> {
        while offset > self.len() {
            if ! self.blocking_read() {
                break;
            }
        }
        if offset >= self.len() {
            None
        } else {
            let end = (offset + len).min(self.len());
            Some(self.buffer[offset..end].as_bytes())
        }
    }


    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<StreamBuffer> {
        let src = if let Some(file_path) = input_file {
            let file = File::open(file_path)?;
            Source::File(BufReader::new(file))
        } else {
            Source::Stdin
        };

        let file = StreamBuffer {
            buffer: String::default(),
            src
        };

        Ok(file)
    }
}

impl TextLogStream {
    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<TextLogStream> {
        let bfr = StreamBuffer::new(input_file)?;
        Ok(TextLogStream {
            stream: Rc::new(RefCell::new(bfr)),
        })
    }

}

/* From https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin
   This needs work, but seems like the right idea.


use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::{thread, time};

fn main() {
    let stdin_channel = spawn_stdin_channel();
    loop {
        match stdin_channel.try_recv() {
            Ok(key) => println!("Received: {}", key),
            Err(TryRecvError::Empty) => println!("Channel empty"),
            Err(TryRecvError::Disconnected) => panic!("Channel disconnected"),
        }
        sleep(1000);
    }
}

fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}
*/
