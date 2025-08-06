#[derive(Debug, Clone)]
pub enum ResonateError {
    GenericError,
    NetworkError,
    ExecNotFound,
    DirectoryNotFound,
    UnrecognisedHomeDir,
    SQLError,
    AudioStreamError,
    AlreadyExists,
    STDOUTError
}
