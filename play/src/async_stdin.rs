use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;

pub struct AsyncStdin {
    buffer: Vec<u8>,
    rx: Option<Receiver<Vec<u8>>>,
    pos:u64,
}

impl AsyncStdin {
    pub fn new() -> Self {
        let rx = Some(Self::spawn_stdin_channel());
        Self {
            rx,
            buffer: Vec::default(),
            pos: 0,
        }
    }

    pub fn is_eof(&self) -> bool {
        !self.rx.is_some()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn fill_buffer(&mut self) {
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(mut data) => self.buffer.append(&mut data),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.rx = None;
                        break;
                        // panic!("Channel disconnected"),
                    },
                }
            }
        }
    }

    fn spawn_stdin_channel() -> Receiver<Vec<u8>> {
        // Use a bounded channel to prevent stdin from running away from us
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(100);
        let mut buffer = String::new();
        thread::spawn(move || loop {
            buffer.clear();
            // TODO: Read into a Vec<u8> and avoid utf8-validation of the data
            // TODO: Handle data with no line-feeds
            match std::io::stdin().read_line(&mut buffer) {
                Ok(0) => break,  // EOF
                Ok(_) => tx.send(buffer.as_bytes().iter().copied().collect()).unwrap(),
                Err(err) => { eprint!("{:?}", err); break; },
            }
        });
        rx
    }
}

use std::io::Read;
impl Read for AsyncStdin {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.fill_buffer();
        let start = self.pos as usize;
        let len = buf.len().min(self.len().saturating_sub(start));
        if len > 0 {
            let end = start + len;
            buf[..len].copy_from_slice(&self.buffer[start..end]);
            self.pos = self.pos.saturating_add(len as u64);
        }
        Ok(len)
    }
}

use std::io::{Seek, SeekFrom};
impl Seek for AsyncStdin {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let (start, offset) = match pos {
            SeekFrom::Start(n) => (0_i64, n as i64),
            SeekFrom::Current(n) => (self.pos as i64, n),
            SeekFrom::End(n) => (self.len() as i64, n),
        };
        self.pos = (((start as i64).saturating_add(offset)) as u64).min(self.len() as u64);
        Ok(self.pos)
    }
}

// BufReader<AsyncStdin> is unnecessary and results in extra copies. Avoid using it, and just use our impl instead.
impl std::io::BufRead for AsyncStdin {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.fill_buffer();
        Ok(&self.buffer[self.pos as usize..])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64;
    }
}