use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the Arc has not exactly one strong reference")]
    ArcIntoInner,
    #[error("process exit status is not 0: {0}")]
    Process(String),
}
