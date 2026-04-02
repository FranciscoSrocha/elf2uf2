use thiserror::Error;

#[derive(Error, Debug)]
pub enum BinaryError {
    #[error(
        "out of bounds: cursor at position 0x{position:x}, needed {needed} bytes, but buffer length is {buf_len}"
    )]
    OutOfBounds {
        needed: usize,
        position: usize,
        buf_len: usize,
    },

    #[error("cursor overflow: adding {added} to cursor position {position} caused overflow")]
    CursorOverflow { position: usize, added: usize },
}
