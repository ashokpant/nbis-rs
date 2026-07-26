use thiserror::Error;

#[derive(Error, Debug, uniffi::Enum)]
pub enum NbisError {
    #[error("Failed to interpret provided bytes as a PNG/JPEG image")]
    ImageLoadError,
    #[error("Failed to read file at the specified path: {0}")]
    FileReadError(String),
    #[error("quality must be between 0.0 and 1.0, got: {0}")]
    InvalidQuality(f64),
    #[error("An unexpected NBIS error occurred: {0}")]
    UnexpectedError(i64),
    #[error("Template could not be parsed: {0}")]
    InvalidTemplate(String),
    #[error("Generic error: {0}")]
    GenericError(String),
    #[error("Null context provided")]
    Nfiq2NullContext,

    #[error("Failed to create NFIQ2 object")]
    Nfiq2CreateFailed,

    #[error("NFIQ2 computation failed with error code: {0}")]
    Nfiq2ComputeFailed(i32),

    #[error("NBIS returned invalid minutiae data")]
    InvalidMinutiaeData,

    #[error("Coordinate out of ISO range: {0}")]
    CoordinateOutOfRange(String),

    /// Converted from SIGSEGV/SIGBUS/SIGFPE inside guarded mindtct/NFIQ2/SIVV.
    #[error("Native library crash: {0}")]
    NativeCrash(String),
}
