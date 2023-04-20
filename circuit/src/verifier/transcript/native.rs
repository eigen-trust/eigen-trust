use crate::{
	integer::{native::Integer, rns::RnsParams},
	params::RoundParams,
	poseidon::native::sponge::PoseidonSponge,
	verifier::loader::native::{NUM_BITS, NUM_LIMBS},
};
use halo2::{
	halo2curves::{Coordinates, CurveAffine},
	transcript::{
		EncodedChallenge, Transcript as Halo2Transcript, TranscriptRead as Halo2TranscriptRead,
		TranscriptReadBuffer, TranscriptWrite as Halo2TranscriptWrite, TranscriptWriterBuffer,
	},
};
use snark_verifier::{
	loader::native::NativeLoader as NativeSVLoader,
	util::{
		arithmetic::PrimeField,
		transcript::{Transcript, TranscriptRead, TranscriptWrite},
	},
	Error as VerifierError,
};
use std::{
	io::{Error as IoError, ErrorKind, Read, Result as IoResult, Write},
	marker::PhantomData,
};

/// Width of the hasher state used in the transcript
pub const WIDTH: usize = 5;

/// PoseidonRead structure
pub struct PoseidonRead<RD: Read, C: CurveAffine, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	// Reader
	pub(crate) reader: RD,
	// PoseidonSponge
	pub(crate) state: PoseidonSponge<C::Scalar, WIDTH, R>,
	// Loader
	pub(crate) loader: NativeSVLoader,
	// PhantomData
	_p: PhantomData<P>,
}

impl<RD: Read, C: CurveAffine, P, R> PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Create a new PoseidonRead transcript
	pub fn new(reader: RD, loader: NativeSVLoader) -> Self {
		Self { reader, state: PoseidonSponge::new(), loader, _p: PhantomData }
	}
}

impl<RD: Read, C: CurveAffine, P, R> Transcript<C, NativeSVLoader> for PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Returns [`NativeSVLoader`].
	fn loader(&self) -> &NativeSVLoader {
		&self.loader
	}

	/// Squeeze a challenge.
	fn squeeze_challenge(&mut self) -> C::ScalarExt {
		let default = C::Scalar::default();
		self.state.update(&[default]);
		let mut hasher = self.state.clone();
		let val = hasher.squeeze();
		val
	}

	/// Update with an elliptic curve point.
	fn common_ec_point(&mut self, ec_point: &C) -> Result<(), VerifierError> {
		let default = C::Scalar::default();
		self.state.update(&[default]);

		let coordinates = ec_point.coordinates().unwrap();
		let x_coordinate = coordinates.x();
		let y_coordinate = coordinates.y();
		let x = Integer::<_, _, NUM_LIMBS, NUM_BITS, P>::from_w(x_coordinate.clone());
		let y = Integer::<_, _, NUM_LIMBS, NUM_BITS, P>::from_w(y_coordinate.clone());

		self.state.update(&x.limbs);
		self.state.update(&y.limbs);

		Ok(())
	}

	/// Update with a scalar.
	fn common_scalar(&mut self, scalar: &C::ScalarExt) -> Result<(), VerifierError> {
		let default = C::Scalar::default();
		self.state.update(&[default, *scalar]);

		Ok(())
	}
}

impl<RD: Read, C: CurveAffine, P, R> TranscriptRead<C, NativeSVLoader> for PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Read a scalar.
	fn read_scalar(&mut self) -> Result<C::ScalarExt, VerifierError> {
		let mut data = <C::Scalar as PrimeField>::Repr::default();
		self.reader.read_exact(data.as_mut()).map_err(|err| {
			VerifierError::Transcript(
				err.kind(),
				"invalid field element encoding in proof".to_string(),
			)
		})?;

		let scalar = Option::from(C::Scalar::from_repr(data)).ok_or_else(|| {
			VerifierError::Transcript(
				ErrorKind::Other,
				"invalid field element encoding in proof".to_string(),
			)
		})?;
		<Self as Transcript<C, NativeSVLoader>>::common_scalar(self, &scalar)?;

		Ok(scalar)
	}

	/// Read an elliptic curve point.
	fn read_ec_point(&mut self) -> Result<C, VerifierError> {
		let mut compressed = C::Repr::default();
		self.reader.read_exact(compressed.as_mut()).map_err(|err| {
			VerifierError::Transcript(
				err.kind(),
				"invalid field element encoding in proof".to_string(),
			)
		})?;

		let point: C = Option::from(C::from_bytes(&compressed)).ok_or_else(|| {
			VerifierError::Transcript(
				ErrorKind::Other,
				"invalid point encoding in proof".to_string(),
			)
		})?;

		self.common_ec_point(&point)?;

		Ok(point)
	}
}

/// PoseidonWrite structure
pub struct PoseidonWrite<W: Write, C: CurveAffine, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	// Writer
	writer: W,
	// PoseidonSponge
	state: PoseidonSponge<C::Scalar, WIDTH, R>,
	// Loader
	loader: NativeSVLoader,
	// PhantomData
	_p: PhantomData<P>,
}

impl<W: Write, C: CurveAffine, P, R> PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Create a new PoseidonWrite transcript.
	pub fn new(writer: W) -> Self {
		Self { writer, state: PoseidonSponge::new(), loader: NativeSVLoader, _p: PhantomData }
	}
}

impl<W: Write, C: CurveAffine, P, R> Transcript<C, NativeSVLoader> for PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Returns [`NativeSVLoader`].
	fn loader(&self) -> &NativeSVLoader {
		&self.loader
	}

	/// Squeeze a challenge.
	fn squeeze_challenge(&mut self) -> C::ScalarExt {
		let default = C::Scalar::default();
		self.state.update(&[default]);
		let mut hasher = self.state.clone();
		hasher.squeeze()
	}

	/// Update with an elliptic curve point.
	fn common_ec_point(&mut self, ec_point: &C) -> Result<(), VerifierError> {
		let default = C::Scalar::default();
		self.state.update(&[default]);
		let coords: Coordinates<C> = Option::from(ec_point.coordinates()).ok_or_else(|| {
			VerifierError::Transcript(
				ErrorKind::Other,
				"cannot write points at infinity to the transcript".to_string(),
			)
		})?;

		let x: Integer<_, _, NUM_LIMBS, NUM_BITS, P> = Integer::from_w(coords.x().clone());
		let y: Integer<_, _, NUM_LIMBS, NUM_BITS, P> = Integer::from_w(coords.y().clone());

		self.state.update(&x.limbs);
		self.state.update(&y.limbs);

		Ok(())
	}

	/// Update with a scalar.
	fn common_scalar(&mut self, scalar: &C::ScalarExt) -> Result<(), VerifierError> {
		let default = C::Scalar::default();
		self.state.update(&[default, scalar.clone()]);

		Ok(())
	}
}

impl<W: Write, C: CurveAffine, P, R> TranscriptWrite<C> for PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,

	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Write a scalar.
	fn write_scalar(&mut self, scalar: C::Scalar) -> Result<(), VerifierError> {
		<Self as Transcript<C, NativeSVLoader>>::common_scalar(self, &scalar)?;
		let data = scalar.to_repr();
		self.writer.write_all(data.as_ref()).unwrap();

		Ok(())
	}

	/// Write a elliptic curve point.
	fn write_ec_point(&mut self, ec_point: C) -> Result<(), VerifierError> {
		self.common_ec_point(&ec_point)?;
		let compressed = ec_point.to_bytes();
		self.writer.write_all(compressed.as_ref()).unwrap();
		Ok(())
	}
}

// ----- HALO2 TRANSCRIPT TRAIT IMPLEMENTATIONS -----

#[derive(Debug)]
/// ChallangeScalar structure
pub struct ChallengeScalar<C: CurveAffine>(C::Scalar);

impl<C: CurveAffine> EncodedChallenge<C> for ChallengeScalar<C> {
	type Input = C::Scalar;

	/// Get an encoded challenge from a given input challenge.
	fn new(challenge_input: &C::Scalar) -> Self {
		ChallengeScalar(*challenge_input)
	}

	/// Get a scalar field element from an encoded challenge.
	fn get_scalar(&self) -> C::Scalar {
		self.0
	}
}

impl<RD: Read, C: CurveAffine, P, R> Halo2Transcript<C, ChallengeScalar<C>>
	for PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Squeeze an encoded verifier challenge from the transcript.
	fn squeeze_challenge(&mut self) -> ChallengeScalar<C> {
		let scalar = Transcript::squeeze_challenge(self);
		ChallengeScalar::new(&scalar)
	}

	/// Writing the point to the transcript without writing it to the proof,
	/// treating it as a common input.
	fn common_point(&mut self, point: C) -> IoResult<()> {
		let res = self.common_ec_point(&point);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}

	/// Writing the scalar to the transcript without writing it to the proof,
	/// treating it as a common input.
	fn common_scalar(&mut self, scalar: C::Scalar) -> IoResult<()> {
		let res = <Self as Transcript<C, NativeSVLoader>>::common_scalar(self, &scalar);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}
}

impl<RD: Read, C: CurveAffine, P, R> Halo2TranscriptRead<C, ChallengeScalar<C>>
	for PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Read a curve point from the prover.
	fn read_point(&mut self) -> IoResult<C> {
		let res = <Self as TranscriptRead<C, NativeSVLoader>>::read_ec_point(self);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}

	/// Read a curve scalar from the prover.
	fn read_scalar(&mut self) -> IoResult<C::Scalar> {
		let res = <Self as TranscriptRead<C, NativeSVLoader>>::read_scalar(self);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}
}

impl<RD: Read, C: CurveAffine, P, R> TranscriptReadBuffer<RD, C, ChallengeScalar<C>>
	for PoseidonRead<RD, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Initialize a transcript given an input buffer.
	fn init(reader: RD) -> Self {
		Self::new(reader, NativeSVLoader)
	}
}

impl<W: Write, C: CurveAffine, P, R> Halo2Transcript<C, ChallengeScalar<C>>
	for PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Squeeze an encoded verifier challenge from the transcript.
	fn squeeze_challenge(&mut self) -> ChallengeScalar<C> {
		let scalar = Transcript::squeeze_challenge(self);
		ChallengeScalar::new(&scalar)
	}

	/// Squeeze an encoded verifier challenge from the transcript.
	fn common_point(&mut self, point: C) -> IoResult<()> {
		let res = self.common_ec_point(&point);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}

	/// Writing the scalar to the transcript without writing it to the proof,
	/// treating it as a common input.
	fn common_scalar(&mut self, scalar: C::Scalar) -> IoResult<()> {
		let res = <Self as Transcript<C, NativeSVLoader>>::common_scalar(self, &scalar);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}
}

impl<W: Write, C: CurveAffine, P, R> Halo2TranscriptWrite<C, ChallengeScalar<C>>
	for PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Write a curve point to the proof and the transcript.
	fn write_point(&mut self, point: C) -> IoResult<()> {
		let res = <Self as TranscriptWrite<C>>::write_ec_point(self, point);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}

	/// Write a scalar to the proof and the transcript.
	fn write_scalar(&mut self, scalar: C::Scalar) -> IoResult<()> {
		let res = <Self as TranscriptWrite<C>>::write_scalar(self, scalar);
		res.map_err(|x| match x {
			VerifierError::Transcript(kind, message) => IoError::new(kind, message),
			_ => IoError::new(ErrorKind::Other, "transcript error".to_string()),
		})
	}
}

impl<W: Write, C: CurveAffine, P, R> TranscriptWriterBuffer<W, C, ChallengeScalar<C>>
	for PoseidonWrite<W, C, P, R>
where
	P: RnsParams<C::Base, C::Scalar, NUM_LIMBS, NUM_BITS>,
	R: RoundParams<C::Scalar, WIDTH>,
{
	/// Initialize a transcript given an output buffer.
	fn init(writer: W) -> Self {
		Self::new(writer)
	}

	/// Conclude the interaction and return the output buffer (writer).
	fn finalize(self) -> W {
		self.writer
	}
}
