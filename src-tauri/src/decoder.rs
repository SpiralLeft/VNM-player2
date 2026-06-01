use std::fmt;

pub trait Decoder: Send {
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize, DecoderError>;
    fn seek(&mut self, time_ms: u64) -> Result<(), DecoderError>;
    fn duration(&self) -> Result<u64, DecoderError>;
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
    fn total_decoded_samples(&self) -> u64;
}

#[derive(Debug)]
pub enum DecoderError {
    Io(std::io::Error),
    Decode(String),
    EndOfStream,
    SeekError(String),
    UnsupportedFormat(String),
}

impl fmt::Display for DecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecoderError::Io(e) => write!(f, "IO error: {}", e),
            DecoderError::Decode(msg) => write!(f, "Decode error: {}", msg),
            DecoderError::EndOfStream => write!(f, "End of stream"),
            DecoderError::SeekError(msg) => write!(f, "Seek error: {}", msg),
            DecoderError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
        }
    }
}

impl std::error::Error for DecoderError {}

impl From<std::io::Error> for DecoderError {
    fn from(e: std::io::Error) -> Self {
        DecoderError::Io(e)
    }
}
