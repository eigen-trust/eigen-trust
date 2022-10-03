//! The module for defining the request-response protocol.

use crate::{
	peer::{
		opinion::{self, Opinion},
		pubkey::Pubkey,
	},
	EigenError, Epoch,
};
use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::request_response::{ProtocolName, RequestResponseCodec};
use std::io::{Error, ErrorKind, Result};

/// EigenTrust protocol struct.
#[derive(Debug, Clone, Default)]
pub struct EigenTrustProtocol {
	version: EigenTrustProtocolVersion,
}

impl EigenTrustProtocol {
	/// Create a new EigenTrust protocol.
	pub fn new() -> Self {
		Self { version: EigenTrustProtocolVersion::V1 }
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
pub enum Request {
	Opinion(Opinion),
	Identify(Pubkey),
}

impl Request {
	/// Get the iter of the request.
	pub fn get_iter(&self) -> Option<u32> {
		match self {
			Self::Opinion(op) => Some(op.iter),
			_ => None,
		}
	}
}

/// The EigenTrust protocol response struct.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
	/// Successful response with an opinion.
	OpinionSuccess,
	/// Successful response with a public key.
	Identify(Pubkey),
	/// Failed response, because of invalid request.
	InvalidRequest,
	/// Failed response, because of the internal error.
	InternalError(EigenError),
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
		&mut self, protocol: &Self::Protocol, io: &mut T,
	) -> Result<Self::Request>
	where
		T: AsyncRead + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let mut buf = [0; 1];
				io.read_exact(&mut buf).await?;
				match buf[0] {
					0 => {
						let mut epoch_buf = [0; 8];
						let mut k_buf = [0; 4];
						let mut op_bytes = [0; 8];
						let mut proof_bytes = Vec::new();
						io.read_exact(&mut epoch_buf).await?;
						io.read_exact(&mut k_buf).await?;
						io.read_exact(&mut op_bytes).await?;
						io.read_to_end(&mut proof_bytes).await?;

						let epoch = Epoch::from_be_bytes(epoch_buf);
						let iter = u32::from_be_bytes(k_buf);
						let op = f64::from_be_bytes(op_bytes);
						let opinion = Opinion::new(epoch, iter, op, proof_bytes);

						Ok(Request::Opinion(opinion))
					},
					1 => {
						let mut pk_buf = [0; 32];
						io.read_exact(&mut pk_buf).await?;
						let pubkey = Pubkey::from_bytes(pk_buf);
						Ok(Request::Identify(pubkey))
					},
					_ => Err(Error::new(ErrorKind::InvalidData, "Invalid request")),
				}
			},
		}
	}

	/// Read the response from the given stream.
	async fn read_response<T>(
		&mut self, protocol: &Self::Protocol, io: &mut T,
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
						// Opinion success
						Ok(Response::OpinionSuccess)
					},
					1 => {
						// Identify
						let mut pubkey_bytes = [0; 32];
						io.read_exact(&mut pubkey_bytes).await?;
						let pubkey = Pubkey::from_bytes(pubkey_bytes);
						Ok(Response::Identify(pubkey))
					},
					2 => Ok(Response::InvalidRequest),
					3 => {
						let mut err_code = [0; 1];
						io.read_exact(&mut err_code).await?;
						let err = EigenError::from(err_code[0]);
						Ok(Response::InternalError(err))
					},
					_ => Err(Error::new(ErrorKind::InvalidData, "Invalid response")),
				};
				response
			},
		}
	}

	/// Write the request to the given stream.
	async fn write_request<T>(
		&mut self, protocol: &Self::Protocol, io: &mut T, req: Self::Request,
	) -> Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				match req {
					Request::Opinion(opinion) => {
						let mut bytes = vec![0];
						bytes.extend_from_slice(&opinion.epoch.to_be_bytes());
						bytes.extend_from_slice(&opinion.iter.to_be_bytes());
						bytes.extend_from_slice(&opinion.op.to_be_bytes());
						bytes.extend_from_slice(&opinion.proof_bytes);
						io.write_all(&bytes).await?;
					},
					Request::Identify(pub_key) => {
						let mut bytes = vec![1];
						bytes.extend_from_slice(&pub_key.to_bytes());
						io.write_all(&bytes).await?;
					},
				}
				Ok(())
			},
		}
	}

	/// Write the response to the given stream.
	async fn write_response<T>(
		&mut self, protocol: &Self::Protocol, io: &mut T, res: Self::Response,
	) -> Result<()>
	where
		T: AsyncWrite + Unpin + Send,
	{
		match protocol.version {
			EigenTrustProtocolVersion::V1 => {
				let mut bytes = Vec::new();
				match res {
					Response::OpinionSuccess => {
						bytes.push(0);
					},
					Response::Identify(pub_key) => {
						bytes.push(1);
						bytes.extend_from_slice(&pub_key.to_bytes());
					},
					Response::InvalidRequest => bytes.push(2),
					Response::InternalError(code) => {
						bytes.push(3);
						bytes.push(code.into());
					},
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
	use crate::constants::*;
	use eigen_trust_circuit::{
		halo2wrong::{
			curves::bn256::Bn256,
			halo2::poly::{commitment::ParamsProver, kzg::commitment::ParamsKZG},
		},
		params::poseidon_bn254_5x5::Params,
		utils::{keygen, random_circuit},
	};
	use rand::thread_rng;

	impl Response {
		pub fn success(self) -> Opinion {
			match self {
				Response::Opinion(opinion) => opinion,
				_ => panic!("Response::success called on invalid response"),
			}
		}
	}

	#[tokio::test]
	async fn should_correctly_write_read_request() {
		let mut codec = EigenTrustCodec::default();
		let mut buf = vec![];
		let epoch = Epoch(1);
		let iter = 1;
		let opinion = Opinion::new(epoch, iter, 1.0, vec![1]);
		let req = Request::Opinion(opinion);
		codec.write_request(&EigenTrustProtocol::default(), &mut buf, req).await.unwrap();

		let mut bytes = vec![0];
		bytes.extend_from_slice(&opinion.epoch.to_be_bytes());
		bytes.extend_from_slice(&opinion.iter.to_be_bytes());
		bytes.extend_from_slice(&opinion.op.to_be_bytes());
		bytes.extend_from_slice(&opinion.proof_bytes);

		assert_eq!(buf, bytes);

		let req =
			codec.read_request(&EigenTrustProtocol::default(), &mut &bytes[..]).await.unwrap();
		assert_eq!(req.get_iter().unwrap(), iter);
	}

	#[tokio::test]
	async fn should_correctly_write_read_success_response() {
		let good_res = Response::OpinionSuccess;

		let mut buf = vec![];
		let mut codec = EigenTrustCodec::default();
		codec.write_response(&EigenTrustProtocol::default(), &mut buf, good_res).await.unwrap();

		let mut bytes = vec![];
		bytes.push(0);

		// compare the written bytes with the expected bytes
		assert_eq!(buf, bytes);

		let read_res =
			codec.read_response(&EigenTrustProtocol::default(), &mut &bytes[..]).await.unwrap();
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
		bytes.push(2);
		assert_eq!(buf, bytes);

		let read_res =
			codec.read_response(&EigenTrustProtocol::default(), &mut &bytes[..]).await.unwrap();

		assert_eq!(read_res, bad_res);
	}

	#[tokio::test]
	async fn should_correctly_write_read_internal_error_response() {
		// Testing internal error
		let bad_res = Response::InternalError(EigenError::InvalidAddress);

		let mut buf = vec![];
		let mut codec = EigenTrustCodec::default();
		codec
			.write_response(&EigenTrustProtocol::default(), &mut buf, bad_res.clone())
			.await
			.unwrap();

		let mut bytes = vec![];
		// 3 is internal error code
		bytes.push(3);
		// 1 is invalid address error code
		bytes.push(1);
		assert_eq!(buf, bytes);

		let read_res =
			codec.read_response(&EigenTrustProtocol::default(), &mut &bytes[..]).await.unwrap();

		assert_eq!(read_res, bad_res);
	}
}
