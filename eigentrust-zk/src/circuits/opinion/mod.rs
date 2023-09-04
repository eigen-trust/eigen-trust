/// Native version of Opinion
pub mod native;

use crate::{
	circuits::dynamic_sets::native::Attestation,
	circuits::HASHER_WIDTH,
	ecc::{generic::EccDefaultChipset, EccEqualConfig},
	ecdsa::{
		AssignedEcdsa, AssignedPublicKey, AssignedSignature, EcdsaChipset, EcdsaConfig,
		SignatureAssigner, UnassignedSignature,
	},
	gadgets::main::{
		IsEqualChipset, IsZeroChipset, MainConfig, MulAddChipset, OrChipset, SelectChipset,
		SubChipset,
	},
	params::{ecc::EccParams, rns::RnsParams},
	Chipset, CommonConfig, FieldExt, HasherChipset, RegionCtx, SpongeHasherChipset,
	UnassignedValue,
};
use halo2::{
	circuit::{AssignedCell, Layouter, Region, Value},
	halo2curves::CurveAffine,
	plonk::Error,
};
use std::marker::PhantomData;

use super::dynamic_sets::native::SignedAttestation;

/// Assigned Attestation structure.
#[derive(Debug, Clone)]
pub struct AssignedAttestation<N: FieldExt> {
	/// Ethereum address of peer being rated
	pub about: AssignedCell<N, N>,
	/// Unique identifier for the action being rated
	pub domain: AssignedCell<N, N>,
	/// Given rating for the action
	pub value: AssignedCell<N, N>,
	/// Optional field for attaching additional information to the attestation
	pub message: AssignedCell<N, N>,
}

impl<N: FieldExt> AssignedAttestation<N> {
	/// Creates a new AssignedAttestation
	pub fn new(
		about: AssignedCell<N, N>, domain: AssignedCell<N, N>, value: AssignedCell<N, N>,
		message: AssignedCell<N, N>,
	) -> Self {
		Self { about, domain, value, message }
	}
}

/// Unassigned Attestation structure.
#[derive(Debug, Clone)]
pub struct UnassignedAttestation<N: FieldExt> {
	/// Ethereum address of peer being rated
	pub about: Value<N>,
	/// Unique identifier for the action being rated
	pub domain: Value<N>,
	/// Given rating for the action
	pub value: Value<N>,
	/// Optional field for attaching additional information to the attestation
	pub message: Value<N>,
}

impl<N: FieldExt> UnassignedAttestation<N> {
	/// Creates a new AssignedAttestation
	pub fn new(about: Value<N>, domain: Value<N>, value: Value<N>, message: Value<N>) -> Self {
		Self { about, domain, value, message }
	}
}

impl<N: FieldExt> From<Attestation<N>> for UnassignedAttestation<N> {
	fn from(att: Attestation<N>) -> Self {
		Self {
			about: Value::known(att.about),
			domain: Value::known(att.domain),
			value: Value::known(att.value),
			message: Value::known(att.message),
		}
	}
}

impl<N: FieldExt> UnassignedValue for UnassignedAttestation<N> {
	fn without_witnesses() -> Self {
		Self {
			about: Value::unknown(),
			domain: Value::unknown(),
			value: Value::unknown(),
			message: Value::unknown(),
		}
	}
}

/// Attestation assigner chipset
pub struct AttestationAssigner<N: FieldExt> {
	att: UnassignedAttestation<N>,
}

impl<N: FieldExt> AttestationAssigner<N> {
	/// Construct new attestation assigner
	pub fn new(att: UnassignedAttestation<N>) -> Self {
		Self { att }
	}
}

impl<N: FieldExt> Chipset<N> for AttestationAssigner<N> {
	type Config = ();
	type Output = AssignedAttestation<N>;

	fn synthesize(
		self, common: &CommonConfig, _: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "assigner",
			|region: Region<'_, N>| {
				let mut ctx = RegionCtx::new(region, 0);

				let about = ctx.assign_advice(common.advice[0], self.att.about)?;
				let domain = ctx.assign_advice(common.advice[1], self.att.domain)?;
				let value = ctx.assign_advice(common.advice[2], self.att.value)?;
				let message = ctx.assign_advice(common.advice[3], self.att.message)?;

				Ok(AssignedAttestation::new(about, domain, value, message))
			},
		)
	}
}

/// AssignedSignedAttestation structure.
#[derive(Debug, Clone)]
pub struct AssignedSignedAttestation<
	C: CurveAffine,
	N: FieldExt,
	const NUM_LIMBS: usize,
	const NUM_BITS: usize,
	P,
> where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	// Attestation
	attestation: AssignedAttestation<N>,
	// Signature
	signature: AssignedSignature<C, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	AssignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	/// Creates a new AssignedSignedAttestation
	pub fn new(
		attestation: AssignedAttestation<N>,
		signature: AssignedSignature<C, N, NUM_LIMBS, NUM_BITS, P>,
	) -> Self {
		Self { attestation, signature }
	}
}

/// AssignedSignedAttestation structure.
#[derive(Debug, Clone)]
pub struct UnassignedSignedAttestation<
	C: CurveAffine,
	N: FieldExt,
	const NUM_LIMBS: usize,
	const NUM_BITS: usize,
	P,
> where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	// Attestation
	pub(crate) attestation: UnassignedAttestation<N>,
	// Signature
	pub(crate) signature: UnassignedSignature<C, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	/// Creates a new AssignedSignedAttestation
	pub fn new(
		attestation: UnassignedAttestation<N>,
		signature: UnassignedSignature<C, N, NUM_LIMBS, NUM_BITS, P>,
	) -> Self {
		Self { attestation, signature }
	}
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	From<SignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>>
	for UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	fn from(signed_att: SignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>) -> Self {
		Self {
			attestation: UnassignedAttestation::from(signed_att.attestation),
			signature: UnassignedSignature::from(signed_att.signature),
		}
	}
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P> UnassignedValue
	for UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	fn without_witnesses() -> Self {
		Self {
			attestation: UnassignedAttestation::without_witnesses(),
			signature: UnassignedSignature::without_witnesses(),
		}
	}
}

/// Assigner for SignedAttestation
pub struct SignedAttestationAssigner<
	C: CurveAffine,
	N: FieldExt,
	const NUM_LIMBS: usize,
	const NUM_BITS: usize,
	P,
> where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	sig_att: UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	SignedAttestationAssigner<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	/// SignedAttestation Assigner constructor
	pub fn new(sig_att: UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>) -> Self {
		Self { sig_att }
	}
}

impl<C: CurveAffine, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P> Chipset<N>
	for SignedAttestationAssigner<C, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
{
	type Config = ();
	type Output = AssignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>;

	fn synthesize(
		self, common: &CommonConfig, _: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		let att_assigner = AttestationAssigner::new(self.sig_att.attestation);
		let assigned_att =
			att_assigner.synthesize(common, &(), layouter.namespace(|| "att_assigner"))?;

		let sig_assigner = SignatureAssigner::new(self.sig_att.signature);
		let assigned_sig =
			sig_assigner.synthesize(common, &(), layouter.namespace(|| "att_assigner"))?;

		let assigned_sig_att = AssignedSignedAttestation::new(assigned_att, assigned_sig);
		Ok(assigned_sig_att)
	}
}

/// Configuration elements for the circuit are defined here.
#[derive(Debug, Clone)]
pub struct OpinionConfig<F: FieldExt, H, S>
where
	H: HasherChipset<F, HASHER_WIDTH>,
	S: SpongeHasherChipset<F, HASHER_WIDTH>,
{
	ecdsa: EcdsaConfig,
	main: MainConfig,
	ecc_equal: EccEqualConfig,
	hasher: H::Config,
	sponge: S::Config,
}

impl<F: FieldExt, H, S> OpinionConfig<F, H, S>
where
	H: HasherChipset<F, HASHER_WIDTH>,
	S: SpongeHasherChipset<F, HASHER_WIDTH>,
{
	/// Construct a new config
	pub fn new(
		ecdsa: EcdsaConfig, main: MainConfig, ecc_equal: EccEqualConfig, hasher: H::Config,
		sponge: S::Config,
	) -> Self {
		Self { ecdsa, main, ecc_equal, hasher, sponge }
	}
}

/// Constructs a chip for the circuit.
#[derive(Clone)]
pub struct OpinionChipset<
	const NUM_NEIGHBOURS: usize,
	C: CurveAffine,
	N: FieldExt,
	const NUM_LIMBS: usize,
	const NUM_BITS: usize,
	P,
	EC,
	H,
	SH,
> where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::Scalar, N, NUM_LIMBS, NUM_BITS>,
	EC: EccParams<C>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
	H: HasherChipset<N, HASHER_WIDTH>,
	SH: SpongeHasherChipset<N, HASHER_WIDTH>,
{
	/// Domain of the attestations
	domain: AssignedCell<N, N>,
	/// Set of peers
	set: Vec<AssignedCell<N, N>>,
	/// Attestations to verify,
	attestations: Vec<AssignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>>,
	/// Public key of the attester
	public_key: AssignedPublicKey<C, N, NUM_LIMBS, NUM_BITS, P>,
	/// Assigned ecdsa signature data
	sig_data: Vec<AssignedEcdsa<C, N, NUM_LIMBS, NUM_BITS, P, EC>>,
	/// Left shifters for composing integers
	left_shifters: [AssignedCell<N, N>; NUM_LIMBS],
	/// Constructs a phantom data for the hasher.
	_hasher: PhantomData<(H, SH, EC)>,
}

impl<
		const NUM_NEIGHBOURS: usize,
		C: CurveAffine,
		N: FieldExt,
		const NUM_LIMBS: usize,
		const NUM_BITS: usize,
		P,
		EC,
		H,
		SH,
	> OpinionChipset<NUM_NEIGHBOURS, C, N, NUM_LIMBS, NUM_BITS, P, EC, H, SH>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::Scalar, N, NUM_LIMBS, NUM_BITS>,
	EC: EccParams<C>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
	H: HasherChipset<N, HASHER_WIDTH>,
	SH: SpongeHasherChipset<N, HASHER_WIDTH>,
{
	/// Create a new chip.
	pub fn new(
		domain: AssignedCell<N, N>, set: Vec<AssignedCell<N, N>>,
		attestations: Vec<AssignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>>,
		public_key: AssignedPublicKey<C, N, NUM_LIMBS, NUM_BITS, P>,
		sig_data: Vec<AssignedEcdsa<C, N, NUM_LIMBS, NUM_BITS, P, EC>>,
		left_shifters: [AssignedCell<N, N>; NUM_LIMBS],
	) -> Self {
		Self {
			domain,
			set,
			attestations,
			public_key,
			sig_data,
			left_shifters,
			_hasher: PhantomData,
		}
	}
}

impl<
		const NUM_NEIGHBOURS: usize,
		C: CurveAffine,
		N: FieldExt,
		const NUM_LIMBS: usize,
		const NUM_BITS: usize,
		P,
		EC,
		H,
		SH,
	> Chipset<N> for OpinionChipset<NUM_NEIGHBOURS, C, N, NUM_LIMBS, NUM_BITS, P, EC, H, SH>
where
	P: RnsParams<C::Base, N, NUM_LIMBS, NUM_BITS> + RnsParams<C::ScalarExt, N, NUM_LIMBS, NUM_BITS>,
	EC: EccParams<C>,
	C::Base: FieldExt,
	C::ScalarExt: FieldExt,
	H: HasherChipset<N, HASHER_WIDTH>,
	SH: SpongeHasherChipset<N, HASHER_WIDTH>,
{
	type Config = OpinionConfig<N, H, SH>;
	type Output = (Vec<AssignedCell<N, N>>, AssignedCell<N, N>);

	/// Synthesize the circuit.
	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		// TODO: Reconstruct the address from public key and check against the set item at the same position

		let (zero, one) = layouter.assign_region(
			|| "assign_zero_and_one",
			|region: Region<'_, N>| {
				let mut ctx = RegionCtx::new(region, 0);
				let zero = ctx.assign_from_constant(common.advice[0], N::ZERO)?;
				let one = ctx.assign_from_constant(common.advice[1], N::ONE)?;

				Ok((zero, one))
			},
		)?;

		let pk_point = self.public_key.get_inner_point();
		let default_point = EccDefaultChipset::<C, N, NUM_LIMBS, NUM_BITS, P, EC>::new(pk_point);
		let is_pk_default = default_point.synthesize(
			common,
			&config.ecc_equal,
			layouter.namespace(|| "is_pk_default"),
		)?;

		let mut scores = Vec::new();
		let mut hashes = Vec::new();
		for i in 0..NUM_NEIGHBOURS {
			let att = self.attestations[i].clone();

			// Checks equality of the attestation about and set index
			let is_equal_chip =
				IsEqualChipset::new(att.attestation.about.clone(), self.set[i].clone());
			let equality_check_set_about = is_equal_chip.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "is_equal_chipset_set"),
			)?;

			// Checks equality of the attestation about and set index
			let is_equal_chip =
				IsEqualChipset::new(att.attestation.domain.clone(), self.domain.clone());
			let equality_check_domain = is_equal_chip.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "is_att_domain_equal_domain"),
			)?;

			let hash = H::new([
				att.attestation.about.clone(),
				att.attestation.domain.clone(),
				att.attestation.value.clone(),
				att.attestation.message.clone(),
				zero.clone(),
			]);
			let att_hash =
				hash.finalize(common, &config.hasher, layouter.namespace(|| "att_hash"))?;

			let mut compose_msg = zero.clone();
			for j in 0..NUM_LIMBS {
				let mul_add_chipset = MulAddChipset::new(
					self.sig_data[i].msg_hash.limbs[j].clone(),
					self.left_shifters[j].clone(),
					compose_msg,
				);
				compose_msg = mul_add_chipset.synthesize(
					common,
					&config.main,
					layouter.namespace(|| "mul_add"),
				)?;
			}

			// Constraint equality for the msg_hash from hasher and constructor
			// Constraint equality for the set and att.about
			// Constraint equality for the domain and att.domain
			layouter.assign_region(
				|| "constraint equality",
				|region: Region<'_, N>| {
					let mut ctx = RegionCtx::new(region, 0);
					ctx.constrain_equal(att_hash[0].clone(), compose_msg.clone())?;
					ctx.constrain_equal(equality_check_set_about.clone(), one.clone())?;
					ctx.constrain_equal(equality_check_domain.clone(), one.clone())?;

					Ok(())
				},
			)?;

			let chip = EcdsaChipset::new(
				att.signature.clone(),
				self.public_key.clone(),
				self.sig_data[i].clone(),
			);
			let is_valid =
				chip.synthesize(common, &config.ecdsa, layouter.namespace(|| "ecdsa_verify"))?;

			// Get the bit representig if the verification failed
			let inverse_bit = SubChipset::new(one.clone(), is_valid);
			let is_invalid = inverse_bit.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "inverse_bit"),
			)?;

			// Checking address and public keys values if they are default or not (default is zero)
			let is_zero_chip = IsZeroChipset::new(self.set[i].clone());
			let is_default_address = is_zero_chip.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "is_default_pubkey"),
			)?;

			let select_cond = OrChipset::new(is_pk_default.clone(), is_invalid);
			let cond = select_cond.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "pk_or_invalid_sig_chipset"),
			)?;

			// Check if address is default/zero
			let select_cond = OrChipset::new(cond, is_default_address);
			let cond = select_cond.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "pk_or_invalid_sig_chipset"),
			)?;

			// Select chip for attestation score
			let score_select =
				SelectChipset::new(cond.clone(), zero.clone(), att.attestation.value);
			let final_score = score_select.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "select chipset"),
			)?;

			// Select chip for attestation hash
			let hash_select = SelectChipset::new(cond, zero.clone(), att_hash[0].clone());
			let final_hash = hash_select.synthesize(
				common,
				&config.main,
				layouter.namespace(|| "select chipset"),
			)?;

			scores.push(final_score);
			hashes.push(final_hash);
		}

		let mut sponge = SH::init(common, layouter.namespace(|| "sponge"))?;
		sponge.update(&hashes);
		let op_hash = sponge.squeeze(common, &config.sponge, layouter.namespace(|| "squeeze!"))?;

		Ok((scores, op_hash))
	}
}

#[cfg(test)]
mod test {
	use super::native::Opinion;
	use super::{
		OpinionChipset, OpinionConfig, SignedAttestationAssigner, UnassignedSignedAttestation,
	};
	use crate::circuits::dynamic_sets::native::{Attestation, SignedAttestation};
	use crate::circuits::{PoseidonNativeHasher, PoseidonNativeSponge, HASHER_WIDTH};
	use crate::ecc::generic::UnassignedEcPoint;
	use crate::ecc::{
		AuxConfig, EccAddConfig, EccDoubleConfig, EccEqualConfig, EccMulConfig,
		EccTableSelectConfig, EccUnreducedLadderConfig,
	};
	use crate::ecdsa::native::{EcdsaKeypair, PublicKey};
	use crate::ecdsa::{EcdsaAssigner, EcdsaAssignerConfig, EcdsaConfig};
	use crate::ecdsa::{PublicKeyAssigner, UnassignedPublicKey};
	use crate::gadgets::absorb::AbsorbChip;
	use crate::gadgets::set::{SetChip, SetConfig};
	use crate::integer::{
		IntegerAddChip, IntegerDivChip, IntegerEqualConfig, IntegerMulChip, IntegerReduceChip,
		IntegerSubChip, LeftShiftersAssigner, UnassignedInteger,
	};
	use crate::params::ecc::secp256k1::Secp256k1Params;
	use crate::params::hasher::poseidon_bn254_5x5::Params;
	use crate::params::rns::secp256k1::Secp256k1_4_68;
	use crate::poseidon::sponge::{PoseidonSpongeConfig, StatefulSpongeChipset};
	use crate::poseidon::{FullRoundChip, PartialRoundChip, PoseidonChipset, PoseidonConfig};
	use crate::utils::{big_to_fe, fe_to_big, generate_params, prove_and_verify};
	use crate::UnassignedValue;
	use crate::{
		ecc::generic::native::EcPoint,
		gadgets::{
			bits2num::Bits2NumChip,
			main::{MainChip, MainConfig},
		},
		integer::native::Integer,
		Chip, CommonConfig,
	};
	use crate::{Chipset, RegionCtx};
	use halo2::arithmetic::Field;
	use halo2::circuit::{Region, Value};
	use halo2::dev::MockProver;
	use halo2::halo2curves::bn256::Bn256;
	use halo2::halo2curves::ff::PrimeField;
	use halo2::halo2curves::group::Curve;
	use halo2::halo2curves::secp256k1::Secp256k1;
	use halo2::{
		circuit::{Layouter, SimpleFloorPlanner},
		halo2curves::{
			bn256::Fr,
			secp256k1::{Fp, Fq, Secp256k1Affine},
		},
		plonk::{Circuit, ConstraintSystem, Error},
	};
	use itertools::Itertools;

	const DOMAIN: u128 = 42;
	const NUM_NEIGHBOURS: usize = 4;
	type WB = Fp;
	type SecpScalar = Fq;
	type N = Fr;
	type C = Secp256k1Affine;
	const NUM_LIMBS: usize = 4;
	const NUM_BITS: usize = 68;
	type P = Secp256k1_4_68;
	type EC = Secp256k1Params;
	type H = PoseidonNativeHasher;
	type SH = PoseidonNativeSponge;
	type HC = PoseidonChipset<N, HASHER_WIDTH, Params>;
	type SHC = StatefulSpongeChipset<N, HASHER_WIDTH, Params>;

	#[derive(Clone)]
	struct TestConfig {
		common: CommonConfig,
		opinion: OpinionConfig<N, HC, SHC>,
		ecdsa_assigner: EcdsaAssignerConfig,
	}

	impl TestConfig {
		fn new(meta: &mut ConstraintSystem<N>) -> Self {
			let common = CommonConfig::new(meta);
			let main = MainConfig::new(MainChip::configure(&common, meta));
			let bits2num_selector = Bits2NumChip::configure(&common, meta);
			let set_selector = SetChip::configure(&common, meta);
			let set = SetConfig::new(main.clone(), set_selector);

			let integer_reduce_selector =
				IntegerReduceChip::<WB, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_add_selector =
				IntegerAddChip::<WB, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_sub_selector =
				IntegerSubChip::<WB, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_mul_selector =
				IntegerMulChip::<WB, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_div_selector =
				IntegerDivChip::<WB, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_mul_selector_secp_scalar =
				IntegerMulChip::<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_equal = IntegerEqualConfig::new(main.clone(), set.clone());

			let ecc_add = EccAddConfig::new(
				integer_reduce_selector, integer_sub_selector, integer_mul_selector,
				integer_div_selector,
			);
			let ecc_equal = EccEqualConfig::new(main.clone(), integer_equal.clone());
			let ecc_double = EccDoubleConfig::new(
				integer_reduce_selector, integer_add_selector, integer_sub_selector,
				integer_mul_selector, integer_div_selector,
			);

			let ecc_ladder = EccUnreducedLadderConfig::new(
				integer_add_selector, integer_sub_selector, integer_mul_selector,
				integer_div_selector,
			);

			let ecc_table_select = EccTableSelectConfig::new(main.clone());
			let ecc_mul_scalar = EccMulConfig::new(
				ecc_ladder.clone(),
				ecc_add.clone(),
				ecc_double.clone(),
				ecc_table_select,
				bits2num_selector.clone(),
			);

			let ecdsa = EcdsaConfig::new(
				ecc_mul_scalar, ecc_add, integer_equal, integer_reduce_selector,
				integer_mul_selector_secp_scalar,
			);

			let aux = AuxConfig::new(ecc_double);
			let ecdsa_assigner = EcdsaAssignerConfig::new(aux);

			let fr_selector = FullRoundChip::<_, HASHER_WIDTH, Params>::configure(&common, meta);
			let pr_selector = PartialRoundChip::<_, HASHER_WIDTH, Params>::configure(&common, meta);
			let poseidon = PoseidonConfig::new(fr_selector, pr_selector);
			let absorb_selector = AbsorbChip::<_, HASHER_WIDTH>::configure(&common, meta);
			let sponge = PoseidonSpongeConfig::new(poseidon.clone(), absorb_selector);

			let opinion = OpinionConfig::new(ecdsa, main, ecc_equal, poseidon, sponge);
			TestConfig { common, opinion, ecdsa_assigner }
		}
	}

	#[derive(Clone)]
	struct TestOpinionCircuit {
		attestations: Vec<UnassignedSignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>>,
		domain: Value<N>,
		set: Vec<Value<N>>,
		public_key: UnassignedPublicKey<C, N, NUM_LIMBS, NUM_BITS, P, EC>,
		g_as_ecpoint: UnassignedEcPoint<C, N, NUM_LIMBS, NUM_BITS, P, EC>,
		msg_hash: Vec<UnassignedInteger<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>>,
		s_inv: Vec<UnassignedInteger<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>>,
	}

	impl TestOpinionCircuit {
		fn new(
			attestations: Vec<SignedAttestation<C, N, NUM_LIMBS, NUM_BITS, P>>, domain: N,
			set: Vec<N>, public_key: PublicKey<C, N, NUM_LIMBS, NUM_BITS, P, EC>,
			g_as_ecpoint: EcPoint<C, N, NUM_LIMBS, NUM_BITS, P, EC>,
			msg_hash: Vec<Integer<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>>,
			s_inv: Vec<Integer<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>>,
		) -> Self {
			Self {
				attestations: attestations
					.iter()
					.map(|x| UnassignedSignedAttestation::from(x.clone()))
					.collect_vec(),
				domain: Value::known(domain),
				set: set.iter().map(|&x| Value::known(x)).collect_vec(),
				public_key: UnassignedPublicKey::new(public_key),
				g_as_ecpoint: UnassignedEcPoint::from(g_as_ecpoint),
				msg_hash: msg_hash.iter().map(|x| UnassignedInteger::from(x.clone())).collect_vec(),
				s_inv: s_inv.iter().map(|x| UnassignedInteger::from(x.clone())).collect_vec(),
			}
		}
	}

	impl Circuit<N> for TestOpinionCircuit {
		type Config = TestConfig;
		type FloorPlanner = SimpleFloorPlanner;

		fn without_witnesses(&self) -> Self {
			Self {
				attestations: self
					.attestations
					.iter()
					.map(|_| UnassignedSignedAttestation::without_witnesses())
					.collect_vec(),
				domain: Value::unknown(),
				set: self.set.iter().map(|_| Value::unknown()).collect_vec(),
				public_key: UnassignedPublicKey::without_witnesses(),
				g_as_ecpoint: UnassignedEcPoint::without_witnesses(),
				msg_hash: self
					.msg_hash
					.iter()
					.map(|_| UnassignedInteger::without_witnesses())
					.collect_vec(),
				s_inv: self
					.s_inv
					.iter()
					.map(|_| UnassignedInteger::without_witnesses())
					.collect_vec(),
			}
		}

		fn configure(meta: &mut ConstraintSystem<N>) -> TestConfig {
			TestConfig::new(meta)
		}

		fn synthesize(
			&self, config: TestConfig, mut layouter: impl Layouter<N>,
		) -> Result<(), Error> {
			let mut sig_datas = Vec::new();
			for i in 0..NUM_NEIGHBOURS {
				let ecdsa_assigner = EcdsaAssigner::new(
					self.g_as_ecpoint.clone(),
					self.msg_hash[i].clone(),
					self.s_inv[i].clone(),
				);

				let ecdsa_assigner = ecdsa_assigner.synthesize(
					&config.common,
					&config.ecdsa_assigner,
					layouter.namespace(|| "ecdsa assigner"),
				)?;

				sig_datas.push(ecdsa_assigner);
			}

			let (set, domain) = layouter.assign_region(
				|| "assign_set",
				|region: Region<'_, N>| {
					let mut ctx = RegionCtx::new(region, 0);
					let mut set = Vec::new();
					for i in 0..NUM_NEIGHBOURS {
						let assigned_addr =
							ctx.assign_advice(config.common.advice[0], self.set[i])?;
						set.push(assigned_addr);
						ctx.next();
					}
					let domain = ctx.assign_advice(config.common.advice[0], self.domain)?;
					Ok((set, domain))
				},
			)?;

			let mut attestations = Vec::new();
			for i in 0..NUM_NEIGHBOURS {
				let att_assigner = SignedAttestationAssigner::new(self.attestations[i].clone());
				let assigned_att = att_assigner.synthesize(
					&config.common,
					&(),
					layouter.namespace(|| "sig_att assigner"),
				)?;
				attestations.push(assigned_att);
			}

			let left_shifters_assigner =
				LeftShiftersAssigner::<SecpScalar, N, NUM_LIMBS, NUM_BITS, P>::default();
			let left_shifters = left_shifters_assigner.synthesize(
				&config.common,
				&(),
				layouter.namespace(|| "left_shifters"),
			)?;

			let public_key_assigner = PublicKeyAssigner::new(self.public_key.clone());
			let public_key = public_key_assigner.synthesize(
				&config.common,
				&(),
				layouter.namespace(|| "public_key assigner"),
			)?;

			let opinion: OpinionChipset<NUM_NEIGHBOURS, C, N, NUM_LIMBS, NUM_BITS, P, EC, HC, SHC> =
				OpinionChipset::new(
					domain, set, attestations, public_key, sig_datas, left_shifters,
				);
			let (scores, op_hash) = opinion.synthesize(
				&config.common,
				&config.opinion,
				layouter.namespace(|| "opinion"),
			)?;

			for i in 0..NUM_NEIGHBOURS {
				layouter.constrain_instance(scores[i].cell(), config.common.instance, i)?;
			}
			layouter.constrain_instance(op_hash.cell(), config.common.instance, scores.len())?;

			Ok(())
		}
	}

	#[test]
	fn test_opinion() {
		// Test Opinion Chipset
		let rng = &mut rand::thread_rng();
		let keypairs = [(); NUM_NEIGHBOURS]
			.map(|_| EcdsaKeypair::<C, N, NUM_LIMBS, NUM_BITS, P, EC>::generate_keypair(rng));
		let attester = keypairs[0].clone();
		let pks = keypairs.map(|kp| kp.public_key);
		let set = pks.map(|pk| pk.to_address());

		let g = Secp256k1::generator().to_affine();
		let g_as_ecpoint = EcPoint::<C, N, NUM_LIMBS, NUM_BITS, P, EC>::new(
			Integer::from_w(g.x),
			Integer::from_w(g.y),
		);
		let domain = N::from_u128(DOMAIN);

		let mut msg_hashes = Vec::new();
		let mut s_inv = Vec::new();
		let mut attestations = Vec::new();

		// Attestation to the other peers
		for i in 0..NUM_NEIGHBOURS {
			let about = set[i];
			let value = N::random(rng.clone());
			let message = N::ZERO;
			let attestation = Attestation::new(about, domain, value, message);

			let att_hash_n = attestation.hash::<HASHER_WIDTH, PoseidonNativeHasher>();
			let att_hash: SecpScalar = big_to_fe(fe_to_big(att_hash_n));
			let signature = attester.sign(att_hash.clone(), rng);
			let s_inv_fq = big_to_fe::<SecpScalar>(signature.s.value()).invert().unwrap();

			let msg_hash_int = Integer::from_w(att_hash);
			let s_inv_int = Integer::from_w(s_inv_fq);
			let signed_att = SignedAttestation::new(attestation, signature);

			msg_hashes.push(msg_hash_int);
			s_inv.push(s_inv_int);
			attestations.push(signed_att);
		}

		let opinion_native: Opinion<NUM_NEIGHBOURS, C, N, NUM_LIMBS, NUM_BITS, P, EC, H, SH> =
			Opinion::new(attester.public_key.clone(), attestations.clone(), domain);
		let (_, scores, op_hash) = opinion_native.validate(set.to_vec());

		let mut p_ins = Vec::new();
		p_ins.extend(scores);
		p_ins.push(op_hash);
		let circuit = TestOpinionCircuit::new(
			attestations,
			domain,
			set.to_vec(),
			attester.public_key,
			g_as_ecpoint,
			msg_hashes,
			s_inv,
		);
		let k = 18;
		let prover = MockProver::run(k, &circuit, vec![p_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_opinion_prod() {
		// Test Opinion Chipset production
		let rng = &mut rand::thread_rng();
		let keypairs = [(); NUM_NEIGHBOURS]
			.map(|_| EcdsaKeypair::<C, N, NUM_LIMBS, NUM_BITS, P, EC>::generate_keypair(rng));
		let attester = keypairs[0].clone();
		let pks = keypairs.map(|kp| kp.public_key);
		let set = pks.map(|pk| pk.to_address());

		let g = Secp256k1::generator().to_affine();
		let g_as_ecpoint = EcPoint::<C, N, NUM_LIMBS, NUM_BITS, P, EC>::new(
			Integer::from_w(g.x),
			Integer::from_w(g.y),
		);
		let domain = N::from_u128(DOMAIN);

		let mut msg_hashes = Vec::new();
		let mut s_inv = Vec::new();
		let mut attestations = Vec::new();

		// Attestation to the other peers
		for i in 0..NUM_NEIGHBOURS {
			let about = set[i];
			let value = N::random(rng.clone());
			let message = N::ZERO;
			let attestation = Attestation::new(about, domain, value, message);

			let att_hash_n = attestation.hash::<HASHER_WIDTH, PoseidonNativeHasher>();
			let att_hash: SecpScalar = big_to_fe(fe_to_big(att_hash_n));
			let signature = attester.sign(att_hash.clone(), rng);
			let s_inv_fq = big_to_fe::<SecpScalar>(signature.s.value()).invert().unwrap();

			let msg_hash_int = Integer::from_w(att_hash);
			let s_inv_int = Integer::from_w(s_inv_fq);
			let signed_att = SignedAttestation::new(attestation, signature);

			msg_hashes.push(msg_hash_int);
			s_inv.push(s_inv_int);
			attestations.push(signed_att);
		}

		let opinion_native: Opinion<NUM_NEIGHBOURS, C, N, NUM_LIMBS, NUM_BITS, P, EC, H, SH> =
			Opinion::new(attester.public_key.clone(), attestations.clone(), domain);
		let (_, scores, op_hash) = opinion_native.validate(set.to_vec());

		let mut p_ins = Vec::new();
		p_ins.extend(scores);
		p_ins.push(op_hash);
		let circuit = TestOpinionCircuit::new(
			attestations,
			domain,
			set.to_vec(),
			attester.public_key,
			g_as_ecpoint,
			msg_hashes,
			s_inv,
		);
		let k = 18;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res = prove_and_verify::<Bn256, _, _>(params, circuit, &[&p_ins], rng).unwrap();
		assert!(res);
	}
}
