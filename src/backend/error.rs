use std::fmt::Debug;
use std::fmt::Formatter;

pub type E = Box<dyn Debug>;

pub enum ResonateError {
    GenericError(E),
    FileNotFound(E),
    ProcessCrash(E),
    NetworkError(E),
    ExecNotFound(E),
    DirectoryNotFound(E),
    UnrecognisedHomeDir
}

impl Debug for ResonateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::GenericError(e) => write!(f, "Generic error: {e:?}"),
            Self::FileNotFound(e) => write!(f, "Could not open file: {e:?}"),
            Self::ProcessCrash(e) => write!(f, "Process failed: {e:?}"),
            Self::NetworkError(e) => write!(f, "Network error: {e:?}"),
            Self::ExecNotFound(e) => write!(f, "Could not start process: {e:?}"),
            Self::DirectoryNotFound(e) => write!(f, "Directory not found: {e:?}"),
            Self::UnrecognisedHomeDir => write!(f, "Could not find home directory.")
        }
    }
}
