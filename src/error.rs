#[derive(Debug)]
pub enum AutodocsError {
    PreProcessing(String),
    Metadata(String),
}

impl std::error::Error for AutodocsError {}
impl std::fmt::Display for AutodocsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ERROR: {}",
            match self {
                AutodocsError::PreProcessing(err) => format!("pre-processing error: {err}"),
                AutodocsError::Metadata(err) =>
                    format!("failed to parse function or module metadata: {err}"),
            }
        )
    }
}
