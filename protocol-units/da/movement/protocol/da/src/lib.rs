pub mod mock;

use async_stream::try_stream;
use movement_da_util::blob::ir::blob::DaBlob;
use movement_signer::cryptography::Curve;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};
use tracing::{info, warn};

pub type CertificateStream<'a> =
	Pin<Box<dyn Stream<Item = Result<Certificate, DaError>> + Send + 'a>>;
pub type DaBlobStream<'a, C> =
	Pin<Box<dyn Stream<Item = Result<(DaHeight, DaBlob<C>), DaError>> + Send + 'a>>;

/// A height for a blob on the DA.
#[derive(Debug, Clone)]
pub struct DaHeight(u64);

impl DaHeight {
	pub fn new(height: u64) -> Self {
		Self(height)
	}

	pub fn as_u64(&self) -> u64 {
		self.0
	}
}

/// A certificate from consensus indicating a height.
#[derive(Debug, Clone)]
pub enum Certificate {
	Height(u64),
	Nolo,
}

/// Errors thrown by the DA.
#[derive(Debug, thiserror::Error)]
pub enum DaError {
	#[error("blob submission error: {0}")]
	BlobSubmission(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("blobs at height error: {0}")]
	BlobsAtHeight(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("non-fatal blobs at height error: {0}")]
	NonFatalBlobsAtHeight(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("certificate error: {0}")]
	Certificate(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("non-fatal certificate error: {0}")]
	NonFatalCertificate(#[source] Box<dyn std::error::Error + Send + Sync>),
	#[error("internal error: {0}")]
	Internal(String),
}

/// Trait for DA operations.
pub trait DaOperations<C>: Send + Sync
where
	C: Curve + Send + Sync + 'static + std::fmt::Debug,
{
	fn submit_blob(&self, data: DaBlob<C>) -> impl Future<Output = Result<(), DaError>> + Send;

	fn get_da_blobs_at_height(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<DaBlob<C>>, DaError>> + Send;

	fn get_da_blobs_at_height_for_stream(
		&self,
		height: u64,
	) -> impl Future<Output = Result<Vec<DaBlob<C>>, DaError>> + Send {
		self.get_da_blobs_at_height(height)
	}

	fn stream_certificates(
		&self,
	) -> impl Future<Output = Result<CertificateStream, DaError>> + Send;

	fn stream_da_blobs_between_heights(
		&self,
		start_height: u64,
		end_height: u64,
	) -> impl Future<Output = Result<DaBlobStream<C>, DaError>> + Send {
		info!("streaming DA blobs between heights {} and {}", start_height, end_height);
		async move {
			let stream = try_stream! {
				for height in start_height..end_height {
					let blobs = self.get_da_blobs_at_height_for_stream(height).await?;
					for blob in blobs {
						yield (DaHeight(height), blob);
					}
				}
			};
			Ok(Box::pin(stream) as DaBlobStream<C>)
		}
	}

	fn stream_da_blobs_from_height(
		&self,
		start_height: u64,
	) -> impl Future<Output = Result<DaBlobStream<C>, DaError>> + Send {
		async move {
			let certificate_stream = self.stream_certificates().await?;
			let stream = try_stream! {
				let mut last_height = start_height;
				let mut certificate_stream = certificate_stream;

				while let Some(certificate) = certificate_stream.next().await {

					info!("certificate: {:?}", certificate);

					match certificate {
						Ok(Certificate::Height(height)) if height > last_height => {
							let blob_stream = self
								.stream_da_blobs_between_heights(last_height, height)
								.await?;
							tokio::pin!(blob_stream);

							while let Some(blob) = blob_stream.next().await {
								yield blob?;
							}

							last_height = height;
						}
						Ok(Certificate::Nolo) => {
							// Ignore Nolo
						}
						// Warn log non-fatal certificate errors
						Err(DaError::NonFatalCertificate(e)) => {
							warn!("non-fatal certificate error: {}", e);
						}
						// Exit on all other errors
						Err(e) => {
							yield Err(e)?;
						}
						// If height is less than last height, ignore
						_ => {
							warn!("ignoring certificate");
						}
					}
				}
			};

			Ok(Box::pin(stream) as DaBlobStream<C>)
		}
	}
}
