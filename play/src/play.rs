
use std::{time::{SystemTime, UNIX_EPOCH}, io::Read};
use indexed_file::files::{CachedStreamReader, LogFileUtil};
use std::thread::sleep;
use indexed_file::files::Stream;


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

fn stdin_read_buffered() {
    use std::io::{self, BufRead};

    let stdin = io::stdin();

    let mut prev = millis();
    loop {
        let mut stdin = stdin.lock();
        let buf = stdin.fill_buf().unwrap();
        let bytes = buf.len();

        let now = millis();
        let elapsed = now - prev;
        prev = now;
        println!("{}ms: {} bytes", elapsed, bytes);

        if bytes == 0 {
            break
        }
        stdin.consume(bytes);
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

fn is_seekable(file: &mut dyn std::io::Seek) -> bool {
    use std::io::{SeekFrom::Current};
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


//  (for x in {1..100} ; do echo $x ; sleep 0.1 ; done) | cargo run --bin play
fn try_async_stdin() -> std::io::Result<()> {
    let mut stdin = CachedStreamReader::new(None)?;
    let mut prev = millis();
    let mut prev_len = 0;
    while !stdin.is_eof() {
        stdin.fill_buffer(stdin.len());

        let now = millis();
        let elapsed = now - prev;

        let len = stdin.len();
        if prev_len != len {
            let bytes = len - prev_len;
            let mut buf = vec![0u8; bytes];
            let actual = stdin.read(&mut buf).unwrap();
            assert_eq!(bytes, actual);
            let s = String::from_utf8(buf).unwrap();
            prev_len = len;
            prev = now;
            print!("{elapsed}ms: {s}");
        }
        sleep( std::time::Duration::from_millis(1));
        // print!("{}", String::from_utf8(buf.to_vec()).unwrap());
    }
    Ok(())
}

//  (for x in {1..100} ; do echo $x ; sleep 0.1 ; done) | cargo run --bin play
fn try_async_stdin_terminate_early() -> std::io::Result<()> {
    let mut stdin = CachedStreamReader::new(None)?;
    let mut prev = millis();
    let mut prev_len = 0;
    let mut counter = 0;
    while counter < 10 && !stdin.is_eof() {
        stdin.fill_buffer(stdin.len());

        let now = millis();
        let elapsed = now - prev;

        let len = stdin.len();
        if prev_len != len {
            let bytes = len - prev_len;
            let mut buf = vec![0u8; bytes];
            let actual = stdin.read(&mut buf).unwrap();
            assert_eq!(bytes, actual);
            let s = String::from_utf8(buf).unwrap();
            prev_len = len;
            prev = now;
            counter += 1;
            print!("{elapsed}ms: {s}");
        }
        sleep( std::time::Duration::from_millis(1));
    }
    Ok(())
}

// Seek works in AsyncStdin
fn async_stdin_seek_front_to_back() -> std::io::Result<()>{
    use std::io::{BufRead, Seek, SeekFrom};
    let mut file = CachedStreamReader::new(None)?;

    assert!(is_seekable(&mut file));

    let mut buffer = String::new();
    let mut handle = file;
    // let mut handle = std::io::BufReader::new(handle);

    for _ in 0..4 {
        sleep(std::time::Duration::from_millis(2000));

        handle.seek(SeekFrom::Start(0)).expect("file is seekable");
        let l1 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
        let l2 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
        handle.seek(SeekFrom::End(-50)).expect("file is seekable");
        let _dummy = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
        let _l100 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
        let l100 = { buffer.clear(); handle.read_line(&mut buffer).expect("read_line doesn't fail"); buffer.clone() };
        println!("l1:{l1}l2:{l2}l100:{l100}");
    }
    Result::Ok(())
}


pub fn play() {
    // stdin_read_one_line();
    // stdin_read_all_lines();
    // stdin_read_bytes();
    // stdin_seek_front_to_back().expect("failed");
    // async_stdin_seek_front_to_back().expect("failed");
    // try_async_stdin();
    // try_async_stdin_terminate_early();
    stdin_read_buffered();
}
