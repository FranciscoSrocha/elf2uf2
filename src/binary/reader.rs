use super::{BinaryError, Encoding};

pub struct ByteReader<'a> {
    buf: &'a [u8],
    cursor: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, cursor: 0 }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.cursor)
    }

    pub fn read(&mut self) -> Result<u8, BinaryError> {
        Ok(self.read_array::<1>()?[0])
    }

    fn advance(&mut self, n: usize) -> Result<usize, BinaryError> {
        let end = self
            .cursor
            .checked_add(n)
            .ok_or(BinaryError::CursorOverflow {
                position: self.cursor,
                added: n,
            })?;

        if end > self.len() {
            return Err(BinaryError::OutOfBounds {
                needed: n,
                position: self.cursor,
                buf_len: self.len(),
            });
        }

        let start = self.cursor;
        self.cursor = end;
        Ok(start)
    }

    pub fn skip(&mut self, n: usize) -> Result<(), BinaryError> {
        self.advance(n)?;
        Ok(())
    }

    pub fn seek(&mut self, pos: usize) -> Result<(), BinaryError> {
        if pos > self.buf.len() {
            return Err(BinaryError::OutOfBounds {
                needed: 0,
                position: self.cursor,
                buf_len: self.buf.len(),
            });
        }
        self.cursor = pos;
        Ok(())
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], BinaryError> {
        let start = self.advance(len)?;
        Ok(&self.buf[start..self.cursor])
    }

    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N], BinaryError> {
        let bytes = self.read_bytes(N)?;

        let mut arr = [0u8; N];
        arr.copy_from_slice(bytes);
        Ok(arr)
    }
}

pub struct Reader<'a> {
    inner: ByteReader<'a>,
    encoding: Encoding,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8], encoding: Encoding) -> Self {
        Self {
            inner: ByteReader::new(buf),
            encoding,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    pub fn skip(&mut self, n: usize) -> Result<(), BinaryError> {
        self.inner.skip(n)
    }

    pub fn seek(&mut self, pos: usize) -> Result<(), BinaryError> {
        self.inner.seek(pos)
    }

    pub fn read_u16(&mut self) -> Result<u16, BinaryError> {
        let bytes: [u8; 2] = self.inner.read_array::<2>()?;
        Ok(self.encoding.u16_from_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Result<u32, BinaryError> {
        let bytes = self.inner.read_array::<4>()?;
        Ok(self.encoding.u32_from_bytes(bytes))
    }

    pub fn read_u64(&mut self) -> Result<u64, BinaryError> {
        let bytes = self.inner.read_array::<8>()?;
        Ok(self.encoding.u64_from_bytes(bytes))
    }
}
