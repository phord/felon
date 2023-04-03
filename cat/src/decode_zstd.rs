use ruzstd;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use ruzstd::frame::ReadFrameHeaderError;
use ruzstd::frame_decoder::{BlockDecodingStrategy, FrameDecoderError};
use ruzstd::decoding::block_decoder;
use ruzstd::frame::read_frame_header;

struct FrameInfo{
    physical: u64,
    logical: u64,
    len: u64,
}

pub struct CompressedFile {
    file: File,
    source_bytes: u64,
    decoder: ruzstd::FrameDecoder,

    // Sorted logical->physical file offsets
    frames: Vec<FrameInfo>,

    // Logical position in file
    pos: u64,

    // Logical position in decode stream
    seek_pos: Option<u64>,
}

#[test]
fn test_scan() {
    let f = CompressedFile::new("test.zst");

}
impl CompressedFile {
    pub fn new(path: &str) -> std::io::Result<Self> {
        // TODO: Return error if no file or not known type
        let file = File::open(path)?;
        let source_bytes = file.metadata()?.len();
        let decoder = ruzstd::FrameDecoder::new();

        let mut cf = Self {
            file,
            source_bytes,
            decoder,
            pos: 0,
            seek_pos: None,
            frames: Vec::new(),
        };

        // Read all physical frame sizes into frames.
        cf.scan_frames().expect("File is valid zstd file");

        cf.file.seek(SeekFrom::Start(0)).unwrap();

        Ok(cf)
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

    // Get logical file length, as close as we know
    fn len(&self) -> u64 {
        let last = &self.frames.last().unwrap();
        let len = last.logical + last.len +
            if last.len > 0 { 0 } else {
                // estimate some extra bytes based on remaining compressed data
                assert!(self.source_bytes > last.physical);
                self.source_bytes - last.physical
            };
        len
    }


    fn goto_frame(&mut self, index: usize) {
        let frame = &self.frames[index];

        // Position file to start of frame
        if self.file.stream_position().unwrap() != frame.physical {
            self.file.seek(SeekFrom::Start(frame.physical)).expect("Seek does not fail");
        }
        self.pos = frame.logical;
        self.begin_frame();
    }

    // Update last frame if we just read past the last byte
    fn end_frame(&mut self) {
        let mut frame = self.frames.last_mut().unwrap();
        if frame.len == 0 && self.pos > frame.logical {
            frame.len = self.pos - frame.logical;
            // drop(frame);  Needed?

            // Push a new last-known-frame
            let fpos = self.file.stream_position().unwrap() as u64;
            assert!(fpos > frame.physical);

            self.frames.push(FrameInfo { physical: fpos, logical: self.pos, len: 0 } );
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

            // FIXME: Avoid binary-search lookup if last_frame is still current_frame or current_frame-1
            // Find the frame that holds our target pos
            let index = match self.frames.binary_search_by_key(&self.pos, |f| f.logical) {
                Err(n) => n - 1,
                Ok(n) => n,
            };

            let frame = &self.frames[index];
            let frame_range = frame.logical..frame.logical+frame.len;
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
                // We've reached the end of the frame
                self.end_frame();
                self.begin_frame();
                if self.file.stream_position().unwrap() >= self.source_bytes {
                    // EOF
                    return Ok(())
                }
            } else {
                // Decode more bytes
                match self.decoder.decode_blocks(&mut self.file, BlockDecodingStrategy::UptoBlocks(1)) {
                    Ok(_) => { }
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

impl Seek for CompressedFile {
    fn seek(&mut self, target: SeekFrom) -> Result<u64, std::io::Error> {
        let (start, offset) = match target {
            SeekFrom::Start(n) => (0_i64, n as i64),
            SeekFrom::Current(n) => (self.pos as i64, n),
            SeekFrom::End(n) => (self.source_bytes as i64, n),
        };
        self.pos = (((start as i64).saturating_add(offset)) as u64).min(self.len() as u64);
        Ok(self.pos)
    }
}

impl Read for CompressedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.update_stream()?;

        let result = self.decoder.read(buf);
        if let Ok(bytes) = result {
            self.pos += bytes as u64;
        } else {
        }
        result
    }
}

#[test]
fn test_compressed_file() {
    // HACKY FILENAME
    let file = "/home/phord/git/mine/igrok/test.zst".to_owned();
    let mut comp = CompressedFile::new(&file).unwrap();
    match std::io::copy(&mut comp, &mut std::io::stdout().lock()) {
        Err(e) => eprintln!("Error: {:?}", e),
        Ok(_) => (),
    }
}
