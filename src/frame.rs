// This code has been adapted from https://github.com/NiiightmareXD/windows-capture

#[derive(thiserror::Error, Debug)]
pub enum FrameError {
    #[error("Conversion to vector failed.")]
    FrameConversionFailed,
}

#[derive(Clone)]
pub struct Frame {}

impl Frame {
    pub fn new() -> Frame {
        Self {}
    }
}

impl TryFrom<Frame> for Vec<u8> {
    type Error = FrameError;

    fn try_from(value: Frame) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![0; 0])
    }
}

impl TryFrom<&Frame> for Vec<u8> {
    type Error = FrameError;

    fn try_from(value: &Frame) -> Result<Vec<u8>, Self::Error> {
        Ok(vec![0; 0])
    }
}
