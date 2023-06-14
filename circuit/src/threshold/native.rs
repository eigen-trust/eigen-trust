use crate::{
	params::rns::{compose_big_decimal_f, decompose_big_decimal},
	FieldExt,
};
use halo2::{arithmetic::Field, halo2curves::bn256::Fr};
use num_rational::BigRational;

/// Structure for threshold checks
pub struct Threshold<const NUM_LIMBS: usize, const POWER_OF_TEN: usize> {
	score: Fr,
	ratio: BigRational,
	threshold: Fr,
}

impl<const NUM_LIMBS: usize, const POWER_OF_TEN: usize> Threshold<NUM_LIMBS, POWER_OF_TEN> {
	/// Create new instance
	pub fn new(score: Fr, ratio: BigRational, threshold: Fr) -> Self {
		Self { score, ratio, threshold }
	}

	// TODO: Scale the ratio to the standardised decimal position
	// TODO: Find `NUM_LIMBS` and `POWER_OF_TEN` for standardised decimal position
	/// Method for checking the threshold for a given score
	pub fn check_threshold(&self) -> ThresholdWitness<Fr, NUM_LIMBS> {
		let ratio = self.ratio.clone();

		let num = ratio.numer();
		let den = ratio.denom();

		let num_decomposed =
			decompose_big_decimal::<Fr, NUM_LIMBS, POWER_OF_TEN>(num.to_biguint().unwrap());
		let den_decomposed =
			decompose_big_decimal::<Fr, NUM_LIMBS, POWER_OF_TEN>(den.to_biguint().unwrap());

		// Constraint checks - circuits should implement from this point
		let composed_num_f = compose_big_decimal_f::<Fr, NUM_LIMBS, POWER_OF_TEN>(num_decomposed);
		let composed_den_f = compose_big_decimal_f::<Fr, NUM_LIMBS, POWER_OF_TEN>(den_decomposed);
		let composed_den_f_inv = composed_den_f.invert().unwrap();
		let res_f = composed_num_f * composed_den_f_inv;
		assert!(res_f == self.score);

		// Take the highest POWER_OF_TEN digits for comparison
		// This just means lower precision
		let threshold = self.threshold;
		let first_limb_num = *num_decomposed.last().unwrap();
		let first_limb_den = *den_decomposed.last().unwrap();
		let comp = first_limb_den * threshold;
		let is_bigger = first_limb_num >= comp;

		ThresholdWitness { threshold, is_bigger, num_decomposed, den_decomposed }
	}
}

/// Witness structure for proving threshold checks
pub struct ThresholdWitness<F: FieldExt, const NUM_LIMBS: usize> {
	/// Threshold value to be checked with
	pub threshold: F,
	/// Comparison result
	pub is_bigger: bool,
	/// Target value numerator decomposition
	pub num_decomposed: [F; NUM_LIMBS],
	/// Target value denominator decomposition
	pub den_decomposed: [F; NUM_LIMBS],
}

#[cfg(test)]
mod tests {
	use halo2::halo2curves::ff::PrimeField;
	use num_bigint::BigInt;
	use num_rational::BigRational;
	use num_traits::FromPrimitive;

	use super::*;

	#[test]
	fn test_check_threshold_1() {
		const NUM_LIMBS: usize = 2;
		const POWER_OF_TEN: usize = 3;

		let threshold = 346;
		let num = 345111;
		let den = 1000;

		let comp_u128 = num >= den * threshold;
		println!("comp_u128: {:?}", comp_u128);

		let num_bn = BigInt::from_u128(num).unwrap();
		let den_bn = BigInt::from_u128(den).unwrap();

		let threshold_fr = Fr::from_u128(threshold);
		let num_fr = Fr::from_u128(num);
		let den_fr = Fr::from_u128(den);

		let score = num_fr * den_fr.invert().unwrap();

		let ratio = BigRational::new(num_bn, den_bn);
		let t: Threshold<NUM_LIMBS, POWER_OF_TEN> = Threshold::new(score, ratio, threshold_fr);
		let tw = t.check_threshold();

		assert!(!tw.is_bigger);
	}

	#[test]
	fn test_check_threshold_2() {
		const NUM_LIMBS: usize = 2;
		const POWER_OF_TEN: usize = 3;

		let threshold = 344;
		let num = 345111;
		let den = 1000;

		let comp_u128 = num >= den * threshold;
		println!("comp_u128: {:?}", comp_u128);

		let num_bn = BigInt::from_u128(num).unwrap();
		let den_bn = BigInt::from_u128(den).unwrap();

		let threshold_fr = Fr::from_u128(threshold);
		let num_fr = Fr::from_u128(num);
		let den_fr = Fr::from_u128(den);

		let score = num_fr * den_fr.invert().unwrap();

		let ratio = BigRational::new(num_bn, den_bn);
		let t: Threshold<NUM_LIMBS, POWER_OF_TEN> = Threshold::new(score, ratio, threshold_fr);
		let tw = t.check_threshold();

		assert!(tw.is_bigger);
	}

	#[test]
	fn test_check_threshold_3() {
		const NUM_LIMBS: usize = 5;
		const POWER_OF_TEN: usize = 3;

		let threshold = 346;
		let num = 347123456789123;
		let den = 1984263563965;

		let comp_u128 = num >= den * threshold;
		println!("comp_u128: {:?}", comp_u128);

		let num_bn = BigInt::from_u128(num).unwrap();
		let den_bn = BigInt::from_u128(den).unwrap();

		let threshold_fr = Fr::from_u128(threshold);
		let num_fr = Fr::from_u128(num);
		let den_fr = Fr::from_u128(den);

		let score = num_fr * den_fr.invert().unwrap();

		let ratio = BigRational::new(num_bn, den_bn);
		let t: Threshold<NUM_LIMBS, POWER_OF_TEN> = Threshold::new(score, ratio, threshold_fr);
		let tw = t.check_threshold();

		assert!(tw.is_bigger);
	}
}
