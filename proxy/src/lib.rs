use std::{
    io,
    io::{Error, ErrorKind},
    net::{SocketAddr, ToSocketAddrs},
};
use thiserror::Error;
use tonic::Status;

use token_authenticator::BlockEngineConnectionError;

pub mod deshred;
pub mod forwarder;
pub mod heartbeat;
pub mod server;
pub mod token_authenticator;

#[derive(Debug, Error)]
pub enum ShredstreamProxyError {
    #[error("TonicError {0}")]
    TonicError(#[from] tonic::transport::Error),
    #[error("GrpcError {0}")]
    GrpcError(#[from] Status),
    #[error("ReqwestError {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("SolanaReqwestError {0}")]
    SolanaReqwestError(#[from] solana_client::client_error::reqwest::Error),
    #[error("IoError {0}")]
    IoError(#[from] io::Error),
    #[error("SolanaClientError {0}")]
    SolanaClientError(#[from] solana_client::client_error::ClientError),
    #[error("BincodeError {0}")]
    BincodeError(#[from] bincode::Error),
    #[error("SerdeJsonError {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("RecvError {0}")]
    RecvError(#[from] crossbeam_channel::RecvError),
    #[error("BlockEngineConnectionError {0}")]
    BlockEngineConnectionError(#[from] BlockEngineConnectionError),
    #[error("Generic {0}")]
    Generic(String),
}

pub fn resolve_hostname_port(hostname_port: &str) -> io::Result<(SocketAddr, String)> {
    let socketaddr = hostname_port.to_socket_addrs()?.next().ok_or_else(|| {
        Error::new(
            ErrorKind::AddrNotAvailable,
            format!("Could not find destination {hostname_port}"),
        )
    })?;
    Ok((socketaddr, hostname_port.to_string()))
}