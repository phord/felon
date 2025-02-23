// A sliding buffer to cache recently visited data

// TODO: Make this more efficient with a double-buffer (or chain of buffers)
//       Break the 2nd buffer at EOL to support majority use cases in this project
//       Use BufferedRead::Buffer or https://crates.io/crates/buffer instead for speed?
//       Alternative using file-backed mem buffer: https://crates.io/crates/mmap_buffer

pub(crate) struct ReadBuffer {
    // Buffer for BufRead
    pub(crate) buffer: Vec<u8>,
    pub(crate) start_offset: u64,
    pub(crate) consumed: u64,
}

impl ReadBuffer {
    pub(crate) fn new(start_offset: u64) -> Self {
        ReadBuffer {
            buffer: Vec::default(),
            start_offset,
            consumed: 0,
        }
    }

    pub(crate) fn remaining(&self) -> u64 {
        assert!(self.buffer.len() as u64 >= self.consumed);
        self.buffer.len() as u64 - self.consumed
    }

    pub(crate) fn start(&self) -> u64 {
        self.start_offset
    }

    pub(crate) fn end(&self) -> u64 {
        self.start_offset + self.buffer.len() as u64
    }

    pub(crate) fn pos(&self) -> u64 {
        assert!(self.buffer.len() as u64 >= self.consumed);
        self.start_offset + self.consumed
    }

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    pub(crate) fn get_buffer(&self) -> &[u8] {
        &self.buffer[self.consumed as usize..]
    }

    pub(crate) fn consume(&mut self, amt: u64) {
        self.consumed += amt
    }

    // Extend current buffer by appending to the end.
    pub(crate) fn extend(&mut self, data: Vec<u8>) {
        self.buffer.extend(data);
    }

    pub(crate) fn discard_front(&mut self, amt: u64) {
        assert!(amt as usize <= self.buffer.len());
        assert!(amt <= self.consumed);
        self.buffer = self.buffer[amt as usize..].to_vec();
        self.start_offset += amt;
        self.consumed = self.consumed.saturating_sub(amt);
    }

    /// Move the read position to the given position if it is within the buffer.
    /// return false if pos is not in the buffer.
    pub(crate) fn seek_to(&mut self, pos: u64) -> bool {
        if (self.start()..self.end()).contains(&pos) {
            self.consumed = pos - self.start();
            assert_eq!(pos, self.pos());
            true
        } else {
            false
        }
    }
}
