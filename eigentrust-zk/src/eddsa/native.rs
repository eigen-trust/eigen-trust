use std::marker::PhantomData;

use crate::{
	edwards::{
		native::{Point, UnassignedPoint},
		params::{BabyJubJub, EdwardsParams},
	},
	params::hasher::poseidon_bn254_5x5::Params,
	poseidon::native::Poseidon,
	utils::to_wide,
	UnassignedValue,
};
use halo2::{
	arithmetic::Field,
	circuit::Value,
	halo2curves::{bn256::Fr, ff::FromUniformBytes, group::ff::PrimeField},
};
use num_bigint::BigUint;
use rand::RngCore;

type Hasher = Poseidon<Fr, 5, Params>;

/// Hashes the input with using the BLAKE hash function.
fn blh(b: &[u8]) -> Vec<u8> {
	let mut hash = [0; 64];
	blake::hash(512, b, &mut hash).unwrap();
	hash.to_vec()
}

/// Configures a structure for the secret key.
#[derive(Clone, Debug)]
pub struct SecretKey(Fr, Fr);

impl SecretKey {
	/// Constructs SecretKey from raw values
	pub fn from_raw(sk_raw: [[u8; 32]; 2]) -> Self {
		let part0 = Fr::from_repr(sk_raw[0]).unwrap();
		let part1 = Fr::from_repr(sk_raw[1]).unwrap();
		Self(part0, part1)
	}

	/// Convert to raw bytes
	pub fn to_raw(&self) -> [[u8; 32]; 2] {
		let part0: [u8; 32] = self.0.to_bytes();
		let part1: [u8; 32] = self.1.to_bytes();
		[part0, part1]
	}

	/// Returns a secret key from a byte array.
	/// Used to produce deterministic outputs.
	pub fn from_byte_array(b: &[u8]) -> Self {
		let hash: Vec<u8> = blh(b);
		let bytes_wide = to_wide(&hash[..32]);
		let sk0 = Fr::from_uniform_bytes(&bytes_wide);

		let bytes_wide = to_wide(&hash[32..]);
		let sk1 = Fr::from_uniform_bytes(&bytes_wide);
		SecretKey(sk0, sk1)
	}

	/// Randomly generates a field element and returns
	/// two hashed values from it.
	pub fn random<R: RngCore + Clone>(rng: &mut R) -> Self {
		let a = Fr::random(rng);
		SecretKey::from_byte_array(&a.to_bytes())
	}

	/// Returns a public key from the secret key.
	pub fn public(&self) -> PublicKey {
		let (b8_x, b8_y) = BabyJubJub::b8();
		let b8_point = Point::new(b8_x, b8_y);
		let a = b8_point.mul_scalar(self.0);
		PublicKey(a.affine())
	}
}

/// Configures a structure for the public key.
#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct PublicKey(pub Point<Fr, BabyJubJub>);

impl PublicKey {
	/// Construct PublicKey from raw data
	pub fn from_raw(pk: [[u8; 32]; 2]) -> Self {
		let x = Fr::from_repr(pk[0]).unwrap();
		let y = Fr::from_repr(pk[1]).unwrap();
		let point = Point::new(x, y);
		Self(point)
	}

	/// Convert to raw bytes
	pub fn to_raw(&self) -> [[u8; 32]; 2] {
		let x = self.0.x.to_bytes();
		let y = self.0.y.to_bytes();
		[x, y]
	}
}

/// Configures a structure for the unassigned public key.
#[derive(Clone, Copy, Default, Debug)]
pub struct UnassignedPublicKey(pub UnassignedPoint<Fr, BabyJubJub>);

impl From<PublicKey> for UnassignedPublicKey {
	fn from(pk: PublicKey) -> Self {
		Self(UnassignedPoint { x: Value::known(pk.0.x), y: Value::known(pk.0.y), _p: PhantomData })
	}
}

impl UnassignedValue for UnassignedPublicKey {
	fn without_witnesses() -> Self {
		Self(UnassignedPoint { x: Value::unknown(), y: Value::unknown(), _p: PhantomData })
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Configures signature objects.
pub struct Signature {
	/// Constructs a point for the R.
	pub big_r: Point<Fr, BabyJubJub>,
	/// Constructs a field element for the s.
	pub s: Fr,
}

impl Signature {
	/// Construct signature from the data
	pub fn new(r_x: Fr, r_y: Fr, s: Fr) -> Self {
		let big_r = Point::new(r_x, r_y);
		Self { big_r, s }
	}
}

impl Default for Signature {
	fn default() -> Self {
		let r_x = Fr::zero();
		let r_y = Fr::zero();
		let s = Fr::zero();

		let point = Point::new(r_x, r_y);
		Self { big_r: point, s }
	}
}

#[derive(Clone, Debug)]
/// Configures unassigned signature objects.
pub struct UnassignedSignature {
	/// Constructs a unassigned point for the R.
	pub big_r: UnassignedPoint<Fr, BabyJubJub>,
	/// Constructs a unassigned field element for the s.
	pub s: Value<Fr>,
}

impl From<Signature> for UnassignedSignature {
	fn from(sig: Signature) -> Self {
		Self {
			big_r: UnassignedPoint {
				x: Value::known(sig.big_r.x),
				y: Value::known(sig.big_r.y),
				_p: PhantomData,
			},
			s: Value::known(sig.s),
		}
	}
}

impl UnassignedValue for UnassignedSignature {
	fn without_witnesses() -> Self {
		Self {
			big_r: UnassignedPoint { x: Value::unknown(), y: Value::unknown(), _p: PhantomData },
			s: Value::unknown(),
		}
	}
}

/// Returns a signature from given keys and message.
pub fn sign(sk: &SecretKey, pk: &PublicKey, m: Fr) -> Signature {
	let inputs = [Fr::zero(), sk.1, m, Fr::zero(), Fr::zero()];
	let r = Hasher::new(inputs).permute()[0];
	let r_bn = BigUint::from_bytes_le(&r.to_bytes());

	// R = B8 * r
	let (b8_x, b8_y) = BabyJubJub::b8();
	let b8_point = Point::new(b8_x, b8_y);
	let big_r = b8_point.mul_scalar(r).affine();
	// H(R || PK || M)
	let m_hash_input = [big_r.x, big_r.y, pk.0.x, pk.0.y, m];
	let m_hash = Hasher::new(m_hash_input).permute()[0];
	let m_hash_bn = BigUint::from_bytes_le(&m_hash.to_bytes());
	// S = r + H(R || PK || M) * sk0   (mod n)
	let sk0 = BigUint::from_bytes_le(&sk.0.to_bytes());
	let s = r_bn + &sk0 * m_hash_bn;
	let suborder = BabyJubJub::suborder();
	let s = s % BigUint::from_bytes_le(&suborder.to_bytes());
	let s = Fr::from_uniform_bytes(&to_wide(&s.to_bytes_le()));

	Signature { big_r, s }
}

/// Checks if the signature holds with the given PK and message.
pub fn verify(sig: &Signature, pk: &PublicKey, m: Fr) -> bool {
	let suborder = BabyJubJub::suborder();
	if sig.s > suborder {
		// S can't be higher than SUBORDER
		return false;
	}
	// Cl = s * G
	let (b8_x, b8_y) = BabyJubJub::b8();
	let b8_point = Point::new(b8_x, b8_y);
	let cl = b8_point.mul_scalar(sig.s);
	// H(R || PK || M)
	let m_hash_input = [sig.big_r.x, sig.big_r.y, pk.0.x, pk.0.y, m];
	let m_hash = Hasher::new(m_hash_input).permute()[0];
	let pk_h = pk.0.mul_scalar(m_hash);
	// Cr = R + H(R || PK || M) * PK
	let cr = sig.big_r.projective().add(&pk_h);
	cr.affine().equals(cl.affine())
}

#[cfg(test)]
mod test {
	use super::*;
	use halo2::halo2curves::group::ff::PrimeField;
	use rand::thread_rng;

	#[test]
	fn should_sign_and_verify() {
		// Testing a valid case.
		let mut rng = thread_rng();

		let sk = SecretKey::random(&mut rng);
		let pk = sk.public();

		let m = Fr::from_str_vartime("123456789012345678901234567890").unwrap();
		let sig = sign(&sk, &pk, m);
		let res = verify(&sig, &pk, m);

		assert!(res);
	}

	#[test]
	fn test_invalid_big_r() {
		// Testing invalid R.
		let mut rng = thread_rng();

		let sk = SecretKey::random(&mut rng);
		let pk = sk.public();

		let inputs = [Fr::zero(), Fr::one(), Fr::one(), Fr::zero(), Fr::zero()];
		let different_r = Hasher::new(inputs).permute()[0];

		let m = Fr::from_str_vartime("123456789012345678901234567890").unwrap();
		let mut sig = sign(&sk, &pk, m);

		let (b8_x, b8_y) = BabyJubJub::b8();
		let b8_point = Point::new(b8_x, b8_y);
		sig.big_r = b8_point.mul_scalar(different_r).affine();
		let res = verify(&sig, &pk, m);

		assert_eq!(res, false);
	}

	#[test]
	fn test_invalid_s() {
		// Testing invalid s.
		let mut rng = thread_rng();

		let sk = SecretKey::random(&mut rng);
		let pk = sk.public();

		let m = Fr::from_str_vartime("123456789012345678901234567890").unwrap();
		let mut sig = sign(&sk, &pk, m);
		sig.s = sig.s.add(&Fr::from(1));
		let res = verify(&sig, &pk, m);

		assert_eq!(res, false);
	}

	#[test]
	fn test_invalid_pk() {
		// Testing invalid public key.
		let mut rng = thread_rng();

		let sk1 = SecretKey::random(&mut rng);
		let pk1 = sk1.public();

		let sk2 = SecretKey::random(&mut rng);
		let pk2 = sk2.public();

		let m = Fr::from_str_vartime("123456789012345678901234567890").unwrap();
		let sig = sign(&sk1, &pk1, m);
		let res = verify(&sig, &pk2, m);

		assert_eq!(res, false);
	}

	#[test]
	fn test_invalid_message() {
		// Testing invalid message.
		let mut rng = thread_rng();

		let sk = SecretKey::random(&mut rng);
		let pk = sk.public();

		let m1 = Fr::from_str_vartime("123456789012345678901234567890").unwrap();
		let sig = sign(&sk, &pk, m1);
		let m2 = Fr::from_str_vartime("123456789012345678901234567890123123").unwrap();
		let res = verify(&sig, &pk, m2);

		assert_eq!(res, false);
	}
}
