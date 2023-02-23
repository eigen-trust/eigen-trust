pub mod att_station;
pub mod utils;

use att_station::{AttestationData as AttData, AttestationStation as AttStation};
use eigen_trust_circuit::{
	calculate_message_hash,
	eddsa::native::{sign, SecretKey},
	halo2::halo2curves::{bn256::Fr as Scalar, FieldExt},
	utils::to_short,
};
use eigen_trust_server::manager::{
	attestation::{Attestation, AttestationData},
	NUM_NEIGHBOURS,
};
use ethers::{abi::Address, prelude::EthDisplay, types::Bytes};
use serde::{Deserialize, Serialize};
use utils::{setup_client, SignerMiddlewareArc};

#[derive(Debug)]
pub enum ClientError {
	DecodeError,
	ParseError,
	TxError,
}

#[derive(Serialize, Deserialize, Debug, EthDisplay, Clone)]
pub struct ClientConfig {
	pub ops: [u128; NUM_NEIGHBOURS],
	pub secret_key: [String; 2],
	pub as_address: String,
	pub mnemonic: String,
	pub ethereum_node_url: String,
}

pub struct EigenTrustClient {
	client: SignerMiddlewareArc,
	config: ClientConfig,
	user_secrets_raw: Vec<[String; 3]>,
}

impl EigenTrustClient {
	pub fn new(config: ClientConfig, user_secrets_raw: Vec<[String; 3]>) -> Self {
		let client = setup_client(&config.mnemonic, &config.ethereum_node_url);
		Self { client, config, user_secrets_raw }
	}

	pub async fn attest(&self) -> Result<(), ClientError> {
		let mut sk_vec = Vec::new();
		for x in &self.user_secrets_raw {
			let sk0_decoded_bytes = bs58::decode(&x[1]).into_vec();
			let sk1_decoded_bytes = bs58::decode(&x[2]).into_vec();

			let sk0_decoded = sk0_decoded_bytes.map_err(|_| ClientError::DecodeError)?;
			let sk1_decoded = sk1_decoded_bytes.map_err(|_| ClientError::DecodeError)?;

			let sk0 = to_short(&sk0_decoded);
			let sk1 = to_short(&sk1_decoded);
			let sk = SecretKey::from_raw([sk0, sk1]);
			sk_vec.push(sk);
		}

		let user_secrets: [SecretKey; NUM_NEIGHBOURS] =
			sk_vec.try_into().map_err(|_| ClientError::DecodeError)?;
		let user_publics = user_secrets.map(|s| s.public());

		let sk0_bytes_vec = bs58::decode(&self.config.secret_key[0]).into_vec();
		let sk1_bytes_vec = bs58::decode(&self.config.secret_key[1]).into_vec();

		let sk0_bytes = sk0_bytes_vec.map_err(|_| ClientError::DecodeError)?;
		let sk1_bytes = sk1_bytes_vec.map_err(|_| ClientError::DecodeError)?;

		let mut sk0: [u8; 32] = [0; 32];
		sk0[..].copy_from_slice(&sk0_bytes);

		let mut sk1: [u8; 32] = [0; 32];
		sk1[..].copy_from_slice(&sk1_bytes);

		let sk = SecretKey::from_raw([sk0, sk1]);
		let pk = sk.public();

		let ops = self.config.ops.map(|x| Scalar::from_u128(x));

		let (pks_hash, message_hash) =
			calculate_message_hash::<NUM_NEIGHBOURS, 1>(user_publics.to_vec(), vec![ops.to_vec()]);

		let sig = sign(&sk, &pk, message_hash[0]);

		let att = Attestation::new(sig, pk, user_publics.to_vec(), ops.to_vec());
		let att_data = AttestationData::from(att);
		let bytes = att_data.to_bytes();

		let as_address_res = self.config.as_address.parse::<Address>();
		let as_address = as_address_res.map_err(|_| ClientError::ParseError)?;
		let as_contract = AttStation::new(as_address, self.client.clone());

		let as_data = AttData(
			Address::zero(),
			pks_hash.to_bytes(),
			Bytes::from(bytes.clone()),
		);
		let as_data_vec = vec![as_data];

		let tx_call = as_contract.attest(as_data_vec);
		let tx_res = tx_call.send();
		let tx = tx_res.await.map_err(|_| ClientError::TxError)?;
		let res = tx.await.map_err(|_| ClientError::TxError)?;

		if let Some(receipt) = res {
			println!("Transaction status: {:?}", receipt.status);
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::{utils::deploy, ClientConfig, EigenTrustClient};
	use eigen_trust_server::manager::NUM_NEIGHBOURS;
	use ethers::utils::Anvil;

	#[tokio::test]
	async fn should_add_attestation() {
		let anvil = Anvil::new().spawn();
		let mnemonic = "test test test test test test test test test test test junk".to_string();
		let node_url = anvil.endpoint();
		let address = deploy(&mnemonic, &node_url).await.unwrap();
		let address_string = format!("{:?}", address);

		let dummy_user = [
			"Alice".to_string(),
			"2L9bbXNEayuRMMbrWFynPtgkrXH1iBdfryRH9Soa8M67".to_string(),
			"9rBeBVtbN2MkHDTpeAouqkMWNFJC6Bxb6bXH9jUueWaF".to_string(),
		];
		let user_secrets_raw = vec![dummy_user; NUM_NEIGHBOURS];

		let config = ClientConfig {
			ops: [200, 200, 200, 200, 200],
			secret_key: [
				"2L9bbXNEayuRMMbrWFynPtgkrXH1iBdfryRH9Soa8M67".to_string(),
				"9rBeBVtbN2MkHDTpeAouqkMWNFJC6Bxb6bXH9jUueWaF".to_string(),
			],
			as_address: address_string,
			mnemonic,
			ethereum_node_url: node_url,
		};

		let et_client = EigenTrustClient::new(config, user_secrets_raw);
		let res = et_client.attest().await;
		assert!(res.is_ok());

		drop(anvil);
	}
}
