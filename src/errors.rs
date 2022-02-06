use std::io;

// We derive `Debug` because all types should probably derive `Debug`.
#[derive(Debug)]
pub enum SonogramError {
    Io(io::Error),
    Hound(hound::Error),

    // Our own errors
    InvalidCodec,
    InvalidChannel,
    InvalidDivisor,
    IncompleteData,
}

impl From<io::Error> for SonogramError {
    fn from(err: io::Error) -> SonogramError {
        SonogramError::Io(err)
    }
}

impl From<hound::Error> for SonogramError {
    fn from(err: hound::Error) -> SonogramError {
        SonogramError::Hound(err)
    }
}
