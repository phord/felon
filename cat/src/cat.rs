use crate::config::Config;
use indexed_file::{line_indexer::LineIndexer, files};
use indexed_file::files::ZstdLogFile;
use std::io::{BufRead, Write};
use std::path::PathBuf;


fn get_files_from_cfg() -> Vec<Option<PathBuf>> {
    let cfg = Config::from_env().expect("Config should not error");
    let mut files:Vec<Option<_>> = cfg.filename.iter().cloned().map(|file| Some(file)).collect();
    if files.is_empty() {
        files.push(None);
    };
    files
}

#[allow(dead_code)]
pub fn cat_cmd() {
    for file in get_files_from_cfg() {
        let mut file = LineIndexer::new(files::new_text_file(file).unwrap());
        // TODO: Open all files at once and iterate them sorted if timestamped
        // TODO: Print lines with colors
        for (line, _start) in file.iter_lines() {
            println!("{line}");
        }
    }
}

#[allow(dead_code)]
pub fn tac_cmd() {
    for file in get_files_from_cfg() {
        let mut file = LineIndexer::new(files::new_text_file(file).unwrap());
        // TODO: Open all files at once and iterate them sorted if timestamped
        // TODO: Print lines with colors
        for (line, _start) in file.iter_lines().rev() {
            println!("{line}");
        }
    }
}

// Copy directly from our file to stdout using io::copy
#[allow(dead_code)]
pub fn copycat_cmd() {
    for file in get_files_from_cfg() {
        let mut file = files::new_text_file(file).expect("File failed to open");
        std::io::copy(&mut file, &mut std::io::stdout()).expect("We don't need error handling");
    }
}

// Iterate using BufRead::lines()
#[allow(dead_code)]
pub fn itercat_cmd() {
    let mut out = std::io::stdout();
    for file in get_files_from_cfg() {
        let file = files::new_text_file(file).expect("File failed to open");
        for line in file.lines() {
            let line = line.unwrap();
            out.write(line.as_bytes()).expect("No errors");
            out.write(b"\n").expect("No errors");
        }
    }
}

// Reverse cat by iterating BufRead::lines().
// BufRead::lines() is not a double-ended iterator, so we have to make a copy in RAM first
// TODO: Implement a double-ended line iterator for IndexedFile and test it here
#[allow(dead_code)]
pub fn rev_itercat_cmd() {
    let mut out = std::io::stdout();
    for file in get_files_from_cfg() {
        let file = files::new_text_file(file).expect("File failed to open");
        let lines = file.lines().map(|x| x.unwrap()).collect::<Vec<_>>();
        eprintln!("Read in {} lines", lines.len());
        for line in lines.iter().rev() {
            out.write(line.as_bytes()).expect("No errors");
            out.write(b"\n").expect("No errors");
        }
    }
}


#[allow(dead_code)]
pub fn bufcat_cmd() {
    let mut out = std::io::stdout();
    let mut buf:Vec<u8> = Vec::new();
    buf.reserve(10 * 1024);
    for file in get_files_from_cfg() {
        if let Ok(mut file) = ZstdLogFile::from_path(&file.unwrap()) {
            loop {
                let bytes = file.read_until(b'\n', &mut buf).expect("No errors");
                if bytes == 0 {
                    break
                }
                out.write(buf.as_slice()).expect("No errors");
            }
        }
    }
}


//#[test]
#[allow(dead_code)]
fn test_itercat() {
    let mut path = PathBuf::new();
    // path.push("/home/phord/git/mine/igrok/indexed_file/resources/test/core.log-2022040423.zst");
    path.push("/home/phord/git/mine/igrok/indexed_file/resources/test/core.log-2022040423");
    let mut out = std::io::stdout();
    let file = files::new_text_file(Some(path)).expect("File failed to open");
    {
        for line in file.lines().map(|x| x.unwrap()).collect::<Vec<_>>().iter().rev() {
            out.write(line.as_bytes()).expect("No errors");
            out.write(b"\n").expect("No errors");
        }
    }
}