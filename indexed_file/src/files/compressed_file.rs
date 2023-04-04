// Currently just zstd compression
use ruzstd;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use ruzstd::frame::ReadFrameHeaderError;
use ruzstd::frame_decoder::{BlockDecodingStrategy, FrameDecoderError};
use ruzstd::decoding::block_decoder;
use ruzstd::frame::read_frame_header;

use crate::files::Stream;

struct FrameInfo{
    physical: u64,
    logical: u64,
    len: u64,
}

pub struct CompressedFile<R> {
    file: R,
    source_bytes: u64,
    decoder: ruzstd::FrameDecoder,

    // Sorted logical->physical file offsets
    frames: Vec<FrameInfo>,

    // The last frame we seeked to
    cur_frame: usize,

    // Logical position in file
    pos: u64,

    // Logical position in decode stream
    seek_pos: Option<u64>,
}

impl<R> CompressedFile<R> {
    // Find a convenient chunk of the file to read around target offset
    pub fn get_chunk(&self, target: usize) -> (usize, usize) {
        let index = match self.frames.binary_search_by_key(&(target as u64), |f| f.logical) {
            Err(n) => n - 1,
            Ok(n) => n,
        };

        let frame = &self.frames[index];
        let end = if frame.len == 0 {
            let size = (self.len() - target) as u64;
            // let size = 128 * 1024;   <-- Also tried this
            frame.logical.max(target as u64) + size
        } else {
            frame.logical + frame.len
        };
        (frame.logical as usize , end as usize)
    }

    fn lookup_frame_index(&self, pos: u64) -> usize {
        // Avoid binary-search lookup if target frame is at current_frame[-1..+2]
        // Why +2?  Why not always +1?
        let start = self.cur_frame.max(1) - 1;
        let end = (start + 4).min(self.frames.len());
        let mut index = end;
        for ind in start..end {
            let frame = &self.frames[ind];
            let frame_range = frame.logical..frame.logical+frame.len;
            if frame_range.contains(&pos) {
                index = ind;
                break;
            }
        }
        if index == end {
            // Search the slow way

            index = match self.frames.binary_search_by_key(&pos, |f| f.logical) {
                Err(n) => n - 1,
                Ok(n) => n,
            };
        }
        index
    }
}

impl<R: Read + Seek> CompressedFile<R> {
    pub fn new(mut file: R) -> std::io::Result<Self> {
        // TODO: Return error if no file or not known type
        // let file = File::open(path)?;
        let source_bytes = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;
        let decoder = ruzstd::FrameDecoder::new();

        let mut cf = Self {
            file,
            source_bytes,
            decoder,
            pos: 0,
            seek_pos: None,
            frames: Vec::new(),
            cur_frame: 0,
        };

        // Read all physical frame sizes into self.frames.
        cf.scan_frames().expect("File is valid zstd file");

        cf.file.seek(SeekFrom::Start(0))?;

        Ok(cf)
    }

    pub fn is_recognized(mut file: R) -> bool {
        if file.seek(SeekFrom::Start(0)).is_err() {
            false
        } else {
            match read_frame_header(&mut file) {
                Ok((frame, _bytes_read)) => {
                    frame.check_valid().is_ok()
                },
                _ => false,
            }
        }
    }

    // Scan all the zstd frame headers in the file and record their positions and sizes
    fn scan_frames(&mut self) -> Result<(), ReadFrameHeaderError> {
        let mut pos = 0;

        let mut fpos = 0;
        while fpos < self.source_bytes {
            // Starting a new frame.  Record details.
            let (uncompressed_bytes, frame_bytes) = self.skip_frame()?;
            match uncompressed_bytes {
                None => {
                    // No point continuing the scan because we don't know the uncompressed size
                    // Leave an empty marker for the last physical frame position
                    let frame = FrameInfo { physical: fpos, logical: pos, len: 0};
                    self.frames.push(frame);
                    break
                },
                Some(0) => { /* Skippable; no action */ },
                Some(size) => {
                    let frame = FrameInfo { physical: fpos, logical: pos, len: size};
                    // eprintln!("Frame @ {fpos} holds {pos} to {}", pos+size);
                    self.frames.push(frame);
                    pos += size;
                }
            }
            fpos += frame_bytes;
            assert_eq!(fpos, self.file.stream_position().unwrap() as u64);
        }
        Ok(())
    }

    fn skip_frame(&mut self) -> Result<(Option<u64>, u64), ReadFrameHeaderError> {
        match read_frame_header(&mut self.file) {
            Err(ReadFrameHeaderError::SkipFrame(_magic_num, skip_size,)) => {
                self.file.seek(SeekFrom::Current(skip_size as i64)).unwrap();
                // Skipped a frame with no uncompressible bytes
                // FIXME: Magic number "4" is the size of the frame header we parsed. read_frame_header should tell us that.
                Ok((Some(0), 4u64 + skip_size as u64))
            }
            Ok((frame, bytes_read)) => {
                // Started a new frame. Skip all the blocks.
                let mut bytes_read = bytes_read as u64;
                let mut block_dec = block_decoder::new();
                loop {
                    let (block_header, block_header_size) = block_dec
                        .read_block_header(&mut self.file)
                        .map_err(FrameDecoderError::FailedToReadBlockHeader).expect("TODO: Map error to some common err");

                    // block_header.decompressed_size is usually filled only after decoding the block  :-(
                    bytes_read += block_header_size as u64;
                    self.file.seek(SeekFrom::Current(block_header.content_size as i64)).unwrap();
                    bytes_read += block_header.content_size as u64;
                    if block_header.last_block {
                        break;
                    }
                }
                if frame.header.descriptor.content_checksum_flag() {
                    self.file.seek(SeekFrom::Current(4)).unwrap();
                    bytes_read += 4;
                }
                // Return the uncompressed size or None if we don't know
                let uncompressed_bytes = match frame.header.frame_content_size() {
                    Ok(size) => Some(size),
                    Err(_) => None,
                };
                Ok((uncompressed_bytes, bytes_read))
            },
            Err(other) => {
                // Some error.  Quit early.
                return Err(other)
            },
        }
    }

    fn goto_frame(&mut self, index: usize) {
        let frame = &self.frames[index];

        // Position file to start of frame
        if self.file.stream_position().unwrap() != frame.physical {
            self.file.seek(SeekFrom::Start(frame.physical)).expect("Seek does not fail");
        }
        self.pos = frame.logical;
        self.begin_frame();
        self.cur_frame = index;
    }

    // Update last frame if we just decoded the last byte
    fn end_frame(&mut self) {
        let mut frame = self.frames.last_mut().unwrap();
        let logical_pos = self.pos + self.decoder.can_collect() as u64;
        if frame.len == 0 && logical_pos > frame.logical {
            frame.len = logical_pos - frame.logical;

            // Push a new last-known-frame
            let fpos = self.file.stream_position().unwrap() as u64;
            assert!(fpos > frame.physical);

            if fpos < self.source_bytes {
                self.frames.push(FrameInfo { physical: fpos, logical: logical_pos, len: 0 } );
            }
        }
    }

    fn begin_frame(&mut self) {
        while self.file.stream_position().unwrap() < self.source_bytes {
            match self.decoder.reset(&mut self.file) {
                Err(FrameDecoderError::ReadFrameHeaderError(ReadFrameHeaderError::SkipFrame(
                    _magic_num,
                    skip_size,
                ))) => {
                    self.file.seek(SeekFrom::Current(skip_size as i64)).unwrap();
                    // TODO: If last self.frame points to us, we should move it to point to the next frame instead.
                    continue;
                }
                Ok(_) => {
                    break
                },
                other => {
                    // FIXME: Report this error upstream
                    other.unwrap(); // Report the error and panic
                    break
                },
            }
        }
    }

    fn apply_seek(&mut self) -> Result<(), std::io::Error> {
        if let Some(pos) = self.seek_pos {
            if pos == self.pos {
                // no-op
                return Ok(())
            }

            // Move to a new position

            // Forget this for next time
            self.seek_pos = None;

            let index = self.lookup_frame_index(pos);

            let frame = &self.frames[index];
            let frame_range = frame.logical..frame.logical+frame.len;
            assert!(frame_range.contains(&pos) || frame.len == 0);
            if pos < self.pos || !frame_range.contains(&self.pos) {
                // Open a new frame
                self.goto_frame(index);
            }

            if pos > self.pos {
                // We're in the right frame, but we're behind
                self.skip_bytes(pos - self.pos)?;
            }
        }
        Ok(())
    }

    fn decode_more_bytes(&mut self) -> Result<(), std::io::Error> {
        loop {
            if self.decoder.can_collect() > 0 {
                // You've already got bytes.  Go away.
                return Ok(())
            } else if self.decoder.is_finished() {
                if self.file.stream_position().unwrap() >= self.source_bytes {
                    // EOF
                    return Ok(())
                }
                // Start a new frame
                self.begin_frame();
            } else {
                // Decode more bytes
                match self.decoder.decode_blocks(&mut self.file, BlockDecodingStrategy::UptoBlocks(1)) {
                    Ok(_) => {
                        if self.decoder.is_finished() {
                            // Reached end of frame
                            self.end_frame();
                        }
                    }
                    Err(e) => {
                        let err = std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Error in the zstd decoder: {:?}", e),
                        );
                        return Err(err);
                    }
                }
            }
        }
    }

    fn skip_bytes(&mut self, mut count:u64) -> Result<(), std::io::Error> {
        self.pos += count;
        while count > 0 {
            if count >= self.decoder.can_collect() as u64 {
                count -= self.decoder.can_collect() as u64;
                self.decoder.collect();
                self.decode_more_bytes()?;
            } else {
                // TODO: Avoid allocating and copying to buffer only to skip bytes
                let mut buffer = vec![0u8; count as usize];
                count = 0;
                self.decoder.read(&mut buffer).expect("Read from collected can't fail");
            }
        }
        Ok(())
    }

    // Move the stream position to read more bytes at current logical pos
    fn update_stream(&mut self) -> Result<(), std::io::Error> {
        self.apply_seek()?;
        self.decode_more_bytes()
    }
}

impl<R: Read + Seek> Seek for CompressedFile<R> {
    fn seek(&mut self, target: SeekFrom) -> Result<u64, std::io::Error> {
        // Ideally we could SeekFrom::End(-1000) and only decode the last frame even if we don't know
        // all the frames' decompressed sizes yet. But we wouldn't be able to return the current offset
        // from Start in that case.
        let (start, offset) = match target {
            SeekFrom::Start(n) => (0_i64, n as i64),
            SeekFrom::Current(n) => (self.pos as i64, n),
            SeekFrom::End(n) => (self.source_bytes as i64, n),
        };
        let pos = (((start as i64).saturating_add(offset)) as u64).min(self.len() as u64);
        self.seek_pos = Some(pos);
        // TODO: Actually seek to position and validate it's in range
        Ok(pos)
    }
}

impl<R: Read + Seek> Read for CompressedFile<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.update_stream()?;

        let bytes = self.decoder.read(buf)?;
        self.pos += bytes as u64;
        Ok(bytes)
    }
}

impl<R> Stream for CompressedFile<R> {
    fn len(&self) -> usize {
        let last = &self.frames.last().unwrap();
        let len = last.logical + last.len +
            if last.len > 0 { 0 } else {
                // estimate some extra bytes based on remaining compressed data
                assert!(self.source_bytes >= last.physical);
                self.source_bytes - last.physical
            };
        len as usize
    }
    // Wait on any data at all; Returns true if file is still open
    fn wait(&mut self) -> bool {
        true
    }
}

#[test]
fn test_compressed_file() {
    use std::fs::File;
    // HACKY FILENAME
    let path = "/home/phord/git/mine/igrok/test.zst".to_owned();
    let file = File::open(path).expect("File exists");

    let mut comp = CompressedFile::new(&file).unwrap();
    match std::io::copy(&mut comp, &mut std::io::stdout().lock()) {
        Err(e) => eprintln!("Error: {:?}", e),
        Ok(_) => (),
    }
}


#[test]
fn test_compressed_file_seek() {
    use std::io::{BufRead, BufReader};
    use std::fs::File;
    // HACKY FILENAME
    let path = "/home/phord/git/mine/igrok/test.zst".to_owned();
    let file = File::open(path).expect("File exists");

    let comp = CompressedFile::new(&file).unwrap();
    let mut reader = BufReader::new(comp);
    let mut line6 = String::default();
    let mut first_5_lines = String::default();
    for _ in 0..5 {
        reader.read_line(&mut first_5_lines).expect("Can read 5 lines");
    }
    let count = first_5_lines.len() as u64;
    reader.read_line(&mut line6).expect("Can read 6 lines");

    assert!(!line6.is_empty());

    let mut comp = CompressedFile::new(&file).unwrap();
    comp.seek(SeekFrom::Start(count)).expect("Seek should work");
    let mut reader = BufReader::new(comp);
    let mut line6b = String::default();
    reader.read_line(&mut line6b).expect("Can read 6 lines");

    assert_eq!(line6, line6b);

}

#[test]
fn test_compressed_file_seek_gen() {
    use std::io::{BufRead, BufReader};
}
