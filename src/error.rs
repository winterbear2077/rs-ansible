use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug, Serialize)]
pub enum AnsibleError {
    #[error("SSH connection failed: {0}")]
    SshConnectionError(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Command execution failed: {0}")]
    CommandExecutionError(String),
    
    #[error("Command failed: {0}")]
    CommandError(String),
    
    #[error("File operation failed: {0}")]
    FileOperationError(String),
    
    #[error("System info collection failed: {0}")]
    SystemInfoError(String),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("SSH error: {0}")]
    Ssh2Error(String),
}

impl From<std::io::Error> for AnsibleError {
    fn from(error: std::io::Error) -> Self {
        AnsibleError::IoError(error.to_string())
    }
}

impl From<ssh2::Error> for AnsibleError {
    fn from(error: ssh2::Error) -> Self {
        AnsibleError::Ssh2Error(error.to_string())
    }
}