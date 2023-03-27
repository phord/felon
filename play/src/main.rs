
use std::{time::{SystemTime, UNIX_EPOCH}, io::Read};

fn millis() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let in_ms = since_the_epoch.as_secs() * 1000 +
    since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    in_ms
}

fn stdin_read_one_line() {
    use std::io::{self, BufRead};

    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle.read_line(&mut buffer).expect("read_line doesn't fail");
    println!("Read {} bytes", buffer.len());
}

fn stdin_read_all_lines() {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let handle = stdin.lock();

    let mut prev = millis();
    for line in handle.lines() {
        let now = millis();
        let elapsed = now - prev;
        prev = now;

        println!("{}ms: {}", elapsed, line.unwrap());
    }
}

// Demonstrates that linux is not using line-buffered stdin from processes
// (while true ; do echo -n "test" ; sleep 0.5 ; printf "\n" ; sleep 0.5 ; done) | cargo run --bin play
fn stdin_read_bytes() {
    use std::io;

    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let mut prev = millis();
    let mut buf = [1u8];
    while handle.read_exact(&mut buf).is_ok() {
        let now = millis();
        let elapsed = now - prev;
        prev = now;

        if elapsed > 1 { print!(" [{elapsed}ms] "); }
        print!("{}", String::from_utf8(buf.to_vec()).unwrap());
    }
}

// from https://phrohdoh.com/blog/read-from-file-or-stdin-rust/
// How to anonymize types as traits using Box<dyn Trait>
fn cat() {
    use std::{env, io, fs};
    // file path provided by user
  let input = env::args().nth(1).unwrap();

  // get the generic thing we can read from
  let mut rdr: Box<dyn io::Read> = match input.as_str() {
    "-" => Box::new(io::stdin()),
    _   => Box::new(fs::File::open(input).unwrap()),
  };

  // write data from reader to stdout (or whatever you do with it)
  io::copy(&mut rdr, &mut io::stdout()).unwrap();
}

fn is_seekable(file: &mut std::fs::File) -> bool {
    use std::io::{Seek, SeekFrom::Current};
    file.seek(Current(0)).is_ok()
}

// This fails when taking data from a pipe, as expected
fn stdin_seek_front_to_back() -> std::io::Result<()>{
    use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
    let lock = std::io::stdin().lock();

    #[cfg(any(target_family="unix", target_family="wasi"))]
    let seekable_stdin = unsafe {
        use std::os::unix::io::{AsRawFd, FromRawFd};
        std::fs::File::from_raw_fd(lock.as_raw_fd())
    };

    #[cfg(target_family="windows")]
    let seekable_stdin = unsafe {
        use std::os::windows::io::{AsRawHandle, FromRawHandle};
        std::fs::File::from_raw_handle(lock.as_raw_handle())
    };

    let mut file = seekable_stdin;

    if !is_seekable(&mut file) {
        println!(" STDIN IS NOT SEEKABLE ");
        return Ok(())
    }

    let mut buffer = String::new();
    let mut handle = BufReader::new(file);

    let l1 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
    handle.seek(SeekFrom::Start(0)).expect("file is seekable");
    handle.seek(SeekFrom::End(-5)).expect("file is seekable");
    let l100 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
    let l2 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
    println!("{l1}{l2}{l100}");
    Result::Ok(())
}


// From https://stackoverflow.com/questions/30012995/how-can-i-read-non-blocking-from-stdin


use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::thread::sleep;

struct AsyncStdin {
    buffer: Vec<u8>,
    rx: Option<Receiver<Vec<u8>>>,
}

impl AsyncStdin {
    pub fn new() -> Self {
        let rx = Some(Self::spawn_stdin_channel());
        Self {
            rx,
            buffer: Vec::default(),
        }
    }

    fn is_eof(&self) -> bool {
        !self.rx.is_some()
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn fill_buffer(&mut self) {
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

//  (for x in {1..100} ; do echo $x ; sleep 0.1 ; done) | cargo run --bin play
fn try_async_stdin() {
    let mut stdin = AsyncStdin::new();
    let mut prev = millis();
    let mut prev_len = 0;
    while !stdin.is_eof() {
        stdin.fill_buffer();

        let now = millis();
        let elapsed = now - prev;

        let len = stdin.len();
        if prev_len != len {
            let s = String::from_utf8(stdin.buffer[prev_len..].to_vec()).unwrap();
            prev_len = len;
            prev = now;
            print!("{elapsed}ms: {s}");
        }
        sleep( std::time::Duration::from_millis(1));
        // print!("{}", String::from_utf8(buf.to_vec()).unwrap());
    }
}

//  (for x in {1..100} ; do echo $x ; sleep 0.1 ; done) | cargo run --bin play
fn try_async_stdin_terminate_early() {
    let mut stdin = AsyncStdin::new();
    let mut prev = millis();
    let mut prev_len = 0;
    let mut counter = 0;
    while counter < 10 && !stdin.is_eof() {
        stdin.fill_buffer();

        let now = millis();
        let elapsed = now - prev;

        let len = stdin.len();
        if prev_len != len {
            let s = String::from_utf8(stdin.buffer[prev_len..].to_vec()).unwrap();
            prev_len = len;
            prev = now;
            counter += 1;
            print!("{elapsed}ms: {s}");
        }
        sleep( std::time::Duration::from_millis(1));
    }
}


fn main() {
    // stdin_read_one_line();
    // stdin_read_all_lines();
    // stdin_read_bytes();
    // stdin_seek_front_to_back().expect("failed");
    // try_async_stdin();
    try_async_stdin_terminate_early();
}
