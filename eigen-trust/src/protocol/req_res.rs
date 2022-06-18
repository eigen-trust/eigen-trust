//! The module for defining the request-response protocol.

use crate::{epoch::Epoch, peer::opinion::Opinion};
use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::request_response::{ProtocolName, RequestResponseCodec};
use std::io::Result;

/// EigenTrust protocol struct.
#[derive(Debug, Clone, Default)]
pub struct EigenTrustProtocol {
	version: EigenTrustProtocolVersion,
}

impl EigenTrustProtocol {
	/// Create a new EigenTrust protocol.
	pub fn new() -> Self {
		Self {
			version: EigenTrustProtocolVersion::V1,
		}
	}
}

/// The version of the EigenTrust protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
enum EigenTrustProtocolVersion {
	V1,
}

impl Default for EigenTrustProtocolVersion {
	fn default() -> Self {
		Self::V1
	}
}

/// The EigenTrust protocol codec.
#[derive(Clone, Debug, Default)]
pub struct EigenTrustCodec;

/// The EigenTrust protocol request struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
	epoch: Epoch,
}

impl Request {
	/// Create a new request.
	pub fn new(epoch: Epoch) -> Self {
		Self { epoch }
	}

	/// Get the epoch of the request.
	pub fn get_epoch(&self) -> Epoch {
		self.epoch
	}
}

/// The EigenTrust protocol response struct.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
	/// Successful response with an opinion.
	Success(Opinion),
	/// Failed response, because of invalid request.
	InvalidRequest,
	/// Failed response, because of the internal error.
	InternalError(u8),
}

impl ProtocolName for EigenTrustProtocol {
	/// The name of the protocol.
	fn protocol_name(&self) -> &[u8] {
		match self.version {
			EigenTrustProtocolVersion::V1 => b"/eigen_trust/1.0.0",
		}
	}
}

#[async_trait]
impl RequestResponseCodec for EigenTrustCodec {
	type Protocol = EigenTrustProtocol;
	type Request = Request;
	type Response = Response;

	/// Read the request from the given stream.
	async fn read_request<T>(
		&mut self,
		protocol: &Self::Protocol,
		io: &mut T,
	) -> Result<Self::Request>
	where
		T: AsyncRead + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let mut buf = [0; 8];
				io.read_exact(&mut buf).await?;
				let k = u64::from_be_bytes(buf);
				Ok(Request::new(Epoch(k)))
			},
		}
	}

	/// Read the response from the given stream.
	async fn read_response<T>(
		&mut self,
		protocol: &Self::Protocol,
		io: &mut T,
	) -> Result<Self::Response>
	where
		T: AsyncRead + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let mut buf = [0; 1];
				io.read_exact(&mut buf).await?;
				let response = match buf[0] {
					0 => {
						let mut k_bytes = [0; 8];
						let mut local_trust_score_bytes = [0; 8];
						let mut global_trust_score_bytes = [0; 8];

						io.read_exact(&mut k_bytes).await?;
						io.read_exact(&mut local_trust_score_bytes).await?;
						io.read_exact(&mut global_trust_score_bytes).await?;

						let k = u64::from_be_bytes(k_bytes);
						let local_trust_score = f64::from_be_bytes(local_trust_score_bytes);
						let global_trust_score = f64::from_be_bytes(global_trust_score_bytes);

						let opinion = Opinion::new(Epoch(k), local_trust_score, global_trust_score);

						Response::Success(opinion)
					},
					1 => Response::InvalidRequest,
					other => Response::InternalError(other),
				};
				Ok(response)
			},
		}
	}

	/// Write the request to the given stream.
	async fn write_request<T>(
		&mut self,
		protocol: &Self::Protocol,
		io: &mut T,
		req: Self::Request,
	) -> Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let k = req.get_epoch();
				io.write_all(&k.to_be_bytes()).await?;
				Ok(())
			},
		}
	}

	/// Write the response to the given stream.
	async fn write_response<T>(
		&mut self,
		protocol: &Self::Protocol,
		io: &mut T,
		res: Self::Response,
	) -> Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let mut bytes = Vec::new();
				match res {
					Response::Success(opinion) => {
						bytes.push(0);
						bytes.extend(opinion.k.to_be_bytes());
						bytes.extend(opinion.local_trust_score.to_be_bytes());
						bytes.extend(opinion.global_trust_score.to_be_bytes());
						bytes.extend(opinion.product.to_be_bytes());
					},
					Response::InvalidRequest => bytes.push(1),
					Response::InternalError(code) => bytes.push(code),
				};
				io.write_all(&bytes).await?;
				Ok(())
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	impl Response {
		pub fn success(self) -> Opinion {
			match self {
				Response::Success(opinion) => opinion,
				_ => panic!("Response::success called on invalid response"),
			}
		}
	}

	#[tokio::test]
	async fn should_correctly_write_read_request() {
		let mut codec = EigenTrustCodec::default();
		let mut buf = vec![];
		let epoch = Epoch(1);
		let req = Request::new(epoch);
		codec
			.write_request(&EigenTrustProtocol::default(), &mut buf, req)
			.await
			.unwrap();

		let bytes = epoch.to_be_bytes();
		assert_eq!(buf, bytes);

		let req = codec
			.read_request(&EigenTrustProtocol::default(), &mut &bytes[..])
			.await
			.unwrap();
		assert_eq!(req.get_epoch(), epoch);
	}

	#[tokio::test]
	async fn should_correctly_write_read_success_response() {
		let epoch = Epoch(1);
		let opinion = Opinion::new(epoch, 0.0, 0.0);
		let good_res = Response::Success(opinion);

		let mut buf = vec![];
		let mut codec = EigenTrustCodec::default();
		codec
			.write_response(&EigenTrustProtocol::default(), &mut buf, good_res)
			.await
			.unwrap();

		let mut bytes = vec![];
		bytes.push(0);
		bytes.extend(opinion.k.to_be_bytes());
		bytes.extend(opinion.local_trust_score.to_be_bytes());
		bytes.extend(opinion.global_trust_score.to_be_bytes());
		bytes.extend(opinion.product.to_be_bytes());

		// compare the written bytes with the expected bytes
		assert_eq!(buf, bytes);

		let read_res = codec
			.read_response(&EigenTrustProtocol::default(), &mut &bytes[..])
			.await
			.unwrap();
		assert_eq!(read_res.success(), opinion);
	}

	#[tokio::test]
	async fn should_correctly_write_read_invalid_response() {
		// Testing invalid request
		let bad_res = Response::InvalidRequest;

		let mut buf = vec![];
		let mut codec = EigenTrustCodec::default();
		codec
			.write_response(&EigenTrustProtocol::default(), &mut buf, bad_res.clone())
			.await
			.unwrap();

		let mut bytes = vec![];
		bytes.push(1);
		assert_eq!(buf, bytes);

		let read_res = codec
			.read_response(&EigenTrustProtocol::default(), &mut &bytes[..])
			.await
			.unwrap();

		assert_eq!(read_res, bad_res);
	}

	#[tokio::test]
	async fn should_correctly_write_read_internal_error_response() {
		// Testing internal error
		let bad_res = Response::InternalError(2);

		let mut buf = vec![];
		let mut codec = EigenTrustCodec::default();
		codec
			.write_response(&EigenTrustProtocol::default(), &mut buf, bad_res.clone())
			.await
			.unwrap();

		let mut bytes = vec![];
		bytes.push(2);
		assert_eq!(buf, bytes);

		let read_res = codec
			.read_response(&EigenTrustProtocol::default(), &mut &bytes[..])
			.await
			.unwrap();

		assert_eq!(read_res, bad_res);
	}
}