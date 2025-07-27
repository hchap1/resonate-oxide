#[derive(Debug, Clone)]
pub enum ResonateError {
    GenericError,
    NetworkError,
    ExecNotFound,
    DirectoryNotFound,
    DatabaseCreationError,
    TableCreationError,
    UnrecognisedHomeDir,
    SQLError,
    AudioStreamError,
    AlreadyExists
}
