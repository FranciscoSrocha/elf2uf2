use super::Encoding;

pub struct Writer {
    buf: Vec<u8>,
    encoding: Encoding,
}

impl Writer {
    pub fn with_capacity(capacity: usize, encoding: Encoding) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            encoding,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.len() == 0
    }

    pub fn append_u32(&mut self, val: u32) {
        let bytes = match self.encoding {
            Encoding::LittleEndian => val.to_le_bytes(),
            Encoding::BigEndian => val.to_be_bytes(),
        };

        self.buf.extend_from_slice(&bytes);
    }

    pub fn append_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn into_bytes(self: Writer) -> Vec<u8> {
        self.buf
    }

    pub fn pad_to(&mut self, len: usize) {
        if self.buf.len() < len {
            self.buf.resize(len, 0);
        }
    }
}
