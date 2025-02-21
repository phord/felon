/**
 * CachedStreamReader is a non-blocking stream reader that implements Read, BufRead and Seek. It
 * supports Stdin from a terminal or a redirect, and pipes. A better name might be UnboundedReadBuffer because that
 * is how it functions internally to provide Seek and non-blocking Read.
 *
 * Random seeks are supported by keeping a copy of all the data ever received from stdin in memory. This is possibly
 * wasteful on some systems that already cache the stdin data somewhere, but it can't be helped in any portable way.
 *
 * It is non-blocking because, unlike reading from Stdin, when we try to read past the end of the available data, we
 * do not block waiting for more data to arrive to fulfill the read. Instead, we return a short read (or no bytes) as
 * if we were reading up to the end of the file. If more data does arrive, we will read it into our buffer on some
 * subsequent read.
 *
 * This means that the caller may receive partial data when they expected contiguous chunks. For example, trying to
 * read 100 bytes may return with only 60 bytes even though the next call may return 40 more bytes.
 *
 * Data is spooled into our buffer from a listener thread and results are posted to a mpsc::sync_channel.
 *
 * To prevent runaway source pipes from filling all of RAM needlessly, we use a limited size BufReader and we poll for
 * updates in a bounded channel with a limited queue size. This means that the reader will only read up to
 * QUEUE_SIZE chunks of READ_THRESHOLD bytes to read ahead and we only pull from the queue if the caller wants to read
 * near the end of our currently loaded buffered data. Well-behaved apps will respond to this backpressure to avoid sending
 * more data as well, thus throttling the whole pipeline if needed.
 */

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::{thread, time};

const QUEUE_SIZE:usize = 100;
const READ_THRESHOLD:usize = 10240;

pub trait Stream {
    /// Returns current length of file/stream
    fn len(&self) -> usize;

    fn is_open(&self) -> bool;

    /// Check for more data and update state
    /// Returns new length
    fn poll(&mut self, _timeout: Option<std::time::Instant>) -> usize;

    /// Wait until source is closed/complete
    fn wait_for_end(&mut self) {
        while self.is_open() {
            self.poll(Some(std::time::Instant::now() + time::Duration::from_millis(1000)));
        }
    }

    /// Check if empty
    fn is_empty(&self) -> bool { self.len() == 0 }
}

pub struct CachedStreamReader {
    buffer: Vec<u8>,
    rx: Option<Receiver<Vec<u8>>>,
    pos:u64,
}

impl CachedStreamReader {
    pub fn new(pipe: Option<&PathBuf>) -> std::io::Result<Self> {
        let base = Self {
            rx: None,
            buffer: Vec::default(),
            pos: 0,
        };

        let stream = if let Some(pipe) = pipe {
            Self {
                rx: Some(Self::reader(Some(BufReader::new(File::open(pipe)?)))),
                ..base
            }
        } else {
            let log:Option<BufReader<File>> = None;
            Self {
                rx: Some(Self::reader(log)),
                ..base
            }
        };
        Ok(stream)
    }

    pub fn from_reader<LOG: BufRead + Send + 'static>(pipe: LOG) -> std::io::Result<Self> {
        let mut stream = Self {
            rx: Some(Self::reader(Some(pipe))),
            buffer: Vec::default(),
            pos: 0,
        };

        // Try to init some read
        stream.poll(None);

        Ok(stream)
    }

    pub fn is_eof(&self) -> bool {
        self.rx.is_none()
    }

    // non-blocking read from stream
    fn try_wait(&mut self) -> Option<Vec<u8>> {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(data) => Some(data),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    self.rx = None;
                    None
                },
            }
        } else {
            None
        }
    }

    // Wait on any data at all up to a timeout; returns Some(data) or None if the stream is closed or there was an error
    fn timed_wait(&mut self, timeout: std::time::Instant) -> Option<Vec<u8>> {
        if let Some(rx) = &self.rx {
            let duration = timeout.duration_since(std::time::Instant::now());
            match rx.recv_timeout(duration) {
                Ok(data) => Some(data),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(_) => {
                    self.rx = None;
                    None
                },
            }
        } else {
            None
        }
    }

    pub fn fill_buffer(&mut self, pos: usize) {
        let mut end = self.len();
        while self.is_open() && pos + READ_THRESHOLD > self.len() {
            // FIXME: Magic number timeout
            let bytes = self.poll(Some(std::time::Instant::now() + time::Duration::from_millis(100)));
            if pos < self.len() || bytes == end {
                // We got something, or we timed out trying.
                break
            }
            end = bytes;
        }
    }

    fn reader<LOG: BufRead + Send + 'static>(pipe: Option<LOG>) -> Receiver<Vec<u8>>
    {
        // Use a bounded channel to prevent stdin from running away from us
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(QUEUE_SIZE);
        thread::spawn(move || {
            let mut rdr: Box<dyn std::io::BufRead> = match pipe {
                Some(file) => Box::new(file),
                None => Box::new(std::io::stdin().lock()),
            };
            loop {
                if let Ok(buf) = rdr.as_mut().fill_buf() {
                    let bytes = buf.len();
                    if bytes == 0 {
                        break
                    }

                    let buf = buf.to_vec();
                    if tx.send(buf).is_err() {
                        break  // Broken pipe?
                    }
                    rdr.consume(bytes);
                }
            }
        });
        rx
    }
}

impl Stream for CachedStreamReader {
    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn is_open(&self) -> bool {
        self.rx.is_some()
    }

    // Poll for new data
    fn poll(&mut self, timeout: Option<std::time::Instant>) -> usize {
        while self.is_open() {
            let data = if let Some(timeout) = timeout {
                if std::time::Instant::now() > timeout {
                    break
                }
                self.timed_wait(timeout)
            } else {
                self.try_wait()
            };
            if let Some(mut data) = data {
                self.buffer.append(&mut data);
            } else {
                // queue is drained or we timed out
                break
            }
            if timeout.is_none() {
                // only try once if we weren't given a timeout
                break
            }
        }
        self.buffer.len()
    }
}

impl Read for CachedStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Blocking read
        let start = self.pos as usize;
        self.fill_buffer(start + buf.len());
        let len = buf.len().min(self.len().saturating_sub(start));
        if len > 0 {
            let end = start + len;
            buf[..len].copy_from_slice(&self.buffer[start..end]);
            self.pos = self.pos.saturating_add(len as u64);
        }
        Ok(len)
    }
}

use std::io::SeekFrom;

impl Seek for CachedStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        // There's a dilemma here for stream readers. If we seek to the end, we can't know if more data is coming.
        // So we can choose to either block until the end of the stream, or return the current end position. We
        // choose the latter, but we also ensure that we have read as far as possible in the moment. This will allow
        // us to at least find the current end of the stream, even if more data is coming later.
        // Code that needs to guarantee the end of the stream should call wait_for_end() first.

        if let SeekFrom::End(_) = pos {
            let mut end = self.len();
            while self.is_open() {
                let len = self.poll(None);
                if end == len {
                    // End stopped moving
                    break
                }
                end = len;
            }
        }

        let (start, offset) = match pos {
            SeekFrom::Start(n) => (0_i64, n as i64),
            SeekFrom::Current(n) => (self.pos as i64, n),
            SeekFrom::End(n) => (self.len() as i64, n),
        };
        self.pos = ((start.saturating_add(offset)) as u64).min(self.len() as u64);
        Ok(self.pos)
    }
}

// BufReader<CachedStreamReader> is unnecessary and results in extra copies. Avoid using it, and just use our impl instead.
impl BufRead for CachedStreamReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.fill_buffer(self.pos as usize);
        Ok(&self.buffer[self.pos as usize..])
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64;
    }
}
