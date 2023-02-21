use crate::{ecc::native::EcPoint, integer::rns::RnsParams};
use halo2::halo2curves::CurveAffine;
use snark_verifier::{
	loader::{EcPointLoader, LoadedEcPoint, LoadedScalar, Loader, ScalarLoader},
	util::arithmetic::FieldOps,
	Error as VerifierError,
};
use std::{
	fmt::Debug,
	marker::PhantomData,
	ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// NUM_LIMBS
pub const NUM_LIMBS: usize = 4;
/// NUM_BITS
pub const NUM_BITS: usize = 68;

#[derive(Debug, Default, Clone, PartialEq)]
/// NativeLoader
pub struct NativeLoader<C: CurveAffine, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	_c: PhantomData<C>,
	_p: PhantomData<P>,
}

#[derive(Debug, Default, Clone, PartialEq)]
/// LScalar
pub struct LScalar<C: CurveAffine, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	inner: C::Scalar,
	loader: NativeLoader<C, P>,
}

impl<C: CurveAffine, P> LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	/// new
	pub fn new(value: C::Scalar, loader: NativeLoader<C, P>) -> Self {
		Self { inner: value, loader }
	}
}

impl<C: CurveAffine, P> FieldOps for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn invert(&self) -> Option<Self> {
		// TODO: InvertChip, TIP: Extract from MainGate.IsZeroChipset
		None
	}
}

// ---- ADD ----

impl<'a, C: CurveAffine, P> Add<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn add(self, rhs: &'a LScalar<C, P>) -> Self::Output {
		// TODO: AddChip
		self
	}
}

impl<C: CurveAffine, P> Add<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn add(self, rhs: LScalar<C, P>) -> Self::Output {
		// TODO: AddChip -- reuse from above: add(self, rhs: &'a other)
		self
	}
}

impl<'a, C: CurveAffine, P> AddAssign<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn add_assign(&mut self, rhs: &'a LScalar<C, P>) {
		// TODO: AddChip -- reuse from above: add(self, rhs: &'a other)
	}
}

impl<C: CurveAffine, P> AddAssign<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn add_assign(&mut self, rhs: LScalar<C, P>) {
		// TODO: AddChip -- reuse from above: add(self, rhs: &'a other)
	}
}

// ---- MUL ----

impl<'a, C: CurveAffine, P> Mul<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn mul(self, rhs: &'a LScalar<C, P>) -> Self::Output {
		// TODO: MulChip
		self
	}
}

impl<C: CurveAffine, P> Mul<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn mul(self, rhs: LScalar<C, P>) -> Self::Output {
		// TODO: MulChip -- reuse from above: mul(self, rhs: &'a other)
		self
	}
}

impl<'a, C: CurveAffine, P> MulAssign<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn mul_assign(&mut self, rhs: &'a LScalar<C, P>) {
		// TODO: MulChip -- reuse from above: mul(self, rhs: &'a other)
	}
}

impl<C: CurveAffine, P> MulAssign<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn mul_assign(&mut self, rhs: LScalar<C, P>) {
		// TODO: MulChip -- reuse from above: mul(self, rhs: &'a other)
	}
}

// ---- SUB ----

impl<'a, C: CurveAffine, P> Sub<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn sub(self, rhs: &'a LScalar<C, P>) -> Self::Output {
		// TODO: SubChip
		self
	}
}

impl<C: CurveAffine, P> Sub<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = LScalar<C, P>;

	fn sub(self, rhs: LScalar<C, P>) -> Self::Output {
		// TODO: SubChip -- reuse from above: sub(self, rhs: &'a other)
		self
	}
}

impl<'a, C: CurveAffine, P> SubAssign<&'a LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn sub_assign(&mut self, rhs: &'a LScalar<C, P>) {
		// TODO: SubChip -- reuse from above: sub(self, rhs: &'a other)
	}
}

impl<C: CurveAffine, P> SubAssign<LScalar<C, P>> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	fn sub_assign(&mut self, rhs: LScalar<C, P>) {
		// TODO: SubChip -- reuse from above: sub(self, rhs: &'a other)
	}
}

// ---- NEG ----

impl<C: CurveAffine, P> Neg for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Output = Self;

	fn neg(self) -> Self::Output {
		// TODO: MulChip: multiplication with -1
		self
	}
}

impl<C: CurveAffine, P> LoadedScalar<C::Scalar> for LScalar<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	/// [`Loader`].
	type Loader = NativeLoader<C, P>;

	/// Returns [`Loader`].
	fn loader(&self) -> &Self::Loader {
		&self.loader
	}
}

impl<C: CurveAffine, P> ScalarLoader<C::Scalar> for NativeLoader<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	/// [`LoadedScalar`].
	type LoadedScalar = LScalar<C, P>;

	/// Load a constant field element.
	fn load_const(&self, value: &C::Scalar) -> Self::LoadedScalar {
		// TODO: Assign a value inside a new region and constrain it to be eq to
		// constant
		LScalar::default()
	}

	/// Assert lhs and rhs field elements are equal.
	fn assert_eq(
		&self, annotation: &str, lhs: &Self::LoadedScalar, rhs: &Self::LoadedScalar,
	) -> Result<(), VerifierError> {
		Ok(())
	}
}

#[derive(Debug, Default, Clone, PartialEq)]
/// LEcPoint
pub struct LEcPoint<C: CurveAffine, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	inner: EcPoint<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS, P>,
	loader: NativeLoader<C, P>,
}

impl<C: CurveAffine, P> LoadedEcPoint<C> for LEcPoint<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type Loader = NativeLoader<C, P>;

	/// Returns [`Loader`].
	fn loader(&self) -> &Self::Loader {
		&self.loader
	}
}

impl<C: CurveAffine, P> EcPointLoader<C> for NativeLoader<C, P>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
{
	type LoadedEcPoint = LEcPoint<C, P>;

	fn ec_point_load_const(&self, value: &C) -> Self::LoadedEcPoint {
		LEcPoint::default()
	}

	fn ec_point_assert_eq(
		&self, annotation: &str, lhs: &Self::LoadedEcPoint, rhs: &Self::LoadedEcPoint,
	) -> Result<(), VerifierError> {
		Ok(())
	}

	/// Perform multi-scalar multiplication.
	fn multi_scalar_multiplication(
		pairs: &[(
			&<Self as ScalarLoader<C::Scalar>>::LoadedScalar,
			&Self::LoadedEcPoint,
		)],
	) -> Self::LoadedEcPoint {
		LEcPoint::default()
	}
}

impl<C: CurveAffine, P> Loader<C> for NativeLoader<C, P> where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>
{
}
