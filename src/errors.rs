use std::io;

// We derive `Debug` because all types should probably derive `Debug`.
#[derive(Debug)]
pub enum SonogramError {
    Io(io::Error),
    #[cfg(feature = "hound")]
    Hound(hound::Error),

    // Our own errors
    InvalidCodec,
    InvalidChannel,
    InvalidDivisor,
    IncompleteData,
    InvalidRawDataSize,
}

impl From<io::Error> for SonogramError {
    fn from(err: io::Error) -> SonogramError {
        SonogramError::Io(err)
    }
}

#[cfg(feature = "hound")]
impl From<hound::Error> for SonogramError {
    fn from(err: hound::Error) -> SonogramError {
        SonogramError::Hound(err)
    }
}
