/// Native version of the chip
pub mod native;

use crate::{
	integer::{
		rns::RnsParams, AssignedInteger, IntegerAddChip, IntegerDivChip, IntegerMulChip,
		IntegerReduceChip, IntegerSubChip,
	},
	Chip, Chipset, CommonConfig,
};
use halo2::{
	arithmetic::FieldExt,
	circuit::Layouter,
	plonk::{Error, Selector},
};

/// Structure for the AssignedPoint.
#[derive(Clone)]
struct AssignedPoint<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	// x coordinate of the point
	x: AssignedInteger<W, N, NUM_LIMBS, NUM_BITS, P>,
	// y coordinate of the point
	y: AssignedInteger<W, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	/// Returns a new `AssignedPoint` given its coordinates as `AssignedInteger`
	pub fn new(
		x: AssignedInteger<W, N, NUM_LIMBS, NUM_BITS, P>,
		y: AssignedInteger<W, N, NUM_LIMBS, NUM_BITS, P>,
	) -> AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P> {
		AssignedPoint { x, y }
	}
}

/// Configuration elements for the circuit are defined here.
#[derive(Debug, Clone)]
struct EccAddConfig {
	/// Constructs selectors from different circuits.
	integer_reduce_selector: Selector,
	integer_sub_selector: Selector,
	integer_mul_selector: Selector,
	integer_div_selector: Selector,
}

impl EccAddConfig {
	/// Construct a new config given the selector of child chips
	pub fn new(
		integer_reduce_selector: Selector, integer_sub_selector: Selector,
		integer_mul_selector: Selector, integer_div_selector: Selector,
	) -> Self {
		Self {
			integer_reduce_selector,
			integer_sub_selector,
			integer_mul_selector,
			integer_div_selector,
		}
	}
}

/// Chipset structure for the EccAdd.
struct EccAddChipset<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	// Assigned point p
	p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
	// Assigned point q
	q: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	EccAddChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	/// Creates a new ecc add chipset.
	pub fn new(
		p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
		q: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
	) -> Self {
		Self { p, q }
	}
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P> Chipset<N>
	for EccAddChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	type Config = EccAddConfig;
	type Output = AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>;

	/// Synthesize the circuit.
	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		// Reduce p_x
		let p_x = IntegerReduceChip::new(self.p.x);
		let p_x_reduced = p_x.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_p_x"),
		)?;

		// Reduce p_y
		let p_y = IntegerReduceChip::new(self.p.y);
		let p_y_reduced = p_y.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_p_y"),
		)?;

		// Reduce q_x
		let q_x = IntegerReduceChip::new(self.q.x);
		let q_x_reduced = q_x.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_q_x"),
		)?;

		// Reduce q_y
		let q_y = IntegerReduceChip::new(self.q.y);
		let q_y_reduced = q_y.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_q_y"),
		)?;

		// numerator = q.y.sub(&p.y);
		let numerator_chip = IntegerSubChip::new(q_y_reduced, p_y_reduced.clone());
		let numerator = numerator_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "numerator"),
		)?;

		// denominator = q.x.sub(&p.x);
		let denominator_chip = IntegerSubChip::new(q_x_reduced.clone(), p_x_reduced.clone());
		let denominator = denominator_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "denominator"),
		)?;

		// m = numerator.result.div(&denominator.result)
		let m_chip = IntegerDivChip::new(numerator, denominator);
		let m = m_chip.synthesize(
			&common,
			&config.integer_div_selector,
			layouter.namespace(|| "m"),
		)?;

		// m_squared = m.result.mul(&m.result)
		let m_squared_chip = IntegerMulChip::new(m.clone(), m.clone());
		let m_squared = m_squared_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_squared"),
		)?;

		// m_squared_minus_p_x = m_squared.result.sub(&p.x)
		let m_squared_minus_p_x_chip = IntegerSubChip::new(m_squared, p_x_reduced.clone());
		let m_squared_minus_p_x = m_squared_minus_p_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "m_squared_minus_p_x"),
		)?;

		// r_x = m_squared_minus_p_x.result.sub(&q.x)
		let r_x_chip = IntegerSubChip::new(m_squared_minus_p_x, q_x_reduced.clone());
		let r_x = r_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_x"),
		)?;

		// r_x_minus_p_x = p.x.sub(&r_x.result);
		let r_x_minus_p_x_chip = IntegerSubChip::new(p_x_reduced, r_x.clone());
		let r_x_minus_p_x = r_x_minus_p_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_x_minus_p_x"),
		)?;

		// m_times_r_x_minus_p_x = m.result.mul(&r_x_minus_p_x.result);
		let m_times_r_x_minus_p_x_chip = IntegerMulChip::new(m, r_x_minus_p_x);
		let m_times_r_x_minus_p_x = m_times_r_x_minus_p_x_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_times_r_x_minus_p_x"),
		)?;

		// r_y = m_times_r_x_minus_p_x.result.sub(&p.y)
		let r_y_chip = IntegerSubChip::new(m_times_r_x_minus_p_x, p_y_reduced);
		let r_y = r_y_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_y"),
		)?;

		let r = AssignedPoint::new(r_x, r_y);
		Ok(r)
	}
}

/// Configuration elements for the circuit are defined here.
#[derive(Debug, Clone)]
struct EccDoubleConfig {
	/// Constructs selectors from different circuits.
	integer_reduce_selector: Selector,
	integer_add_selector: Selector,
	integer_sub_selector: Selector,
	integer_mul_selector: Selector,
	integer_div_selector: Selector,
}

impl EccDoubleConfig {
	/// Construct a new config given the selector of child chips
	pub fn new(
		integer_reduce_selector: Selector, integer_add_selector: Selector,
		integer_sub_selector: Selector, integer_mul_selector: Selector,
		integer_div_selector: Selector,
	) -> Self {
		Self {
			integer_reduce_selector,
			integer_add_selector,
			integer_sub_selector,
			integer_mul_selector,
			integer_div_selector,
		}
	}
}

/// Chipset structure for the EccDouble.
struct EccDoubleChipset<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	// Assigned point p
	p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	EccDoubleChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	/// Creates a new ecc double chipset.
	pub fn new(p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>) -> Self {
		Self { p }
	}
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P> Chipset<N>
	for EccDoubleChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	type Config = EccDoubleConfig;
	type Output = AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>;

	/// Synthesize the circuit.
	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		// Reduce p_x
		let p_x = IntegerReduceChip::new(self.p.x);
		let p_x_reduced = p_x.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_p_x"),
		)?;

		// Reduce p_y
		let p_y = IntegerReduceChip::new(self.p.y);
		let p_y_reduced = p_y.synthesize(
			&common,
			&config.integer_reduce_selector,
			layouter.namespace(|| "reduce_p_y"),
		)?;

		// double_p_y = p.y.add(&p.y)
		let double_p_y_chip = IntegerAddChip::new(p_y_reduced.clone(), p_y_reduced.clone());
		let double_p_y = double_p_y_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "double_p_y"),
		)?;

		// p_x_square = p.x.mul(&p.x)
		let p_x_square_chip = IntegerMulChip::new(p_x_reduced.clone(), p_x_reduced.clone());
		let p_x_square = p_x_square_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "p_x_square"),
		)?;

		// p_x_square_times_two = p_x_square.result.add(&p_x_square.result);
		let p_x_square_times_two_chip = IntegerAddChip::new(p_x_square.clone(), p_x_square.clone());
		let p_x_square_times_two = p_x_square_times_two_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "p_x_square_times_two"),
		)?;

		// p_x_square_times_three = p_x_square.result.add(&p_x_square_times_two.result);
		let p_x_square_times_three_chip = IntegerAddChip::new(p_x_square_times_two, p_x_square);
		let p_x_square_times_three = p_x_square_times_three_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "p_x_square_times_three"),
		)?;

		// m = p_x_square_times_three.result.div(&double_p_y.result)
		let m_chip = IntegerDivChip::new(p_x_square_times_three, double_p_y);
		let m = m_chip.synthesize(
			&common,
			&config.integer_div_selector,
			layouter.namespace(|| "m"),
		)?;

		// double_p_x = p.x.add(&p.x)
		let double_p_x_chip = IntegerAddChip::new(p_x_reduced.clone(), p_x_reduced.clone());
		let double_p_x = double_p_x_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "double_p_x"),
		)?;

		// m_squared = m.result.mul(&m.result)
		let m_squared_chip = IntegerMulChip::new(m.clone(), m.clone());
		let m_squared = m_squared_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_squared"),
		)?;

		// r_x = m_squared.result.sub(&double_p_x.result)
		let r_x_chip = IntegerSubChip::new(m_squared, double_p_x);
		let r_x = r_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_x"),
		)?;

		// p_x_minus_r_x = self.x.sub(&r_x.result)
		let p_x_minus_r_x_chip = IntegerSubChip::new(p_x_reduced, r_x.clone());
		let p_x_minus_r_x = p_x_minus_r_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "p_x_minus_r_x"),
		)?;

		// m_times_p_x_minus_r_x = m.result.mul(&p_x_minus_r_x.result)
		let m_times_p_x_minus_r_x_chip = IntegerMulChip::new(m, p_x_minus_r_x);
		let m_times_p_x_minus_r_x = m_times_p_x_minus_r_x_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_times_p_x_minus_r_x"),
		)?;

		// r_y = m_times_p_x_minus_r_x.result.sub(&p.y)
		let r_y_chip = IntegerSubChip::new(m_times_p_x_minus_r_x, p_y_reduced);
		let r_y = r_y_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_y"),
		)?;

		let r = AssignedPoint::new(r_x, r_y);

		Ok(r)
	}
}

/// Configuration elements for the circuit are defined here.
#[derive(Debug, Clone)]
struct EccUnreducedLadderConfig {
	/// Constructs selectors from different circuits.
	integer_add_selector: Selector,
	integer_sub_selector: Selector,
	integer_mul_selector: Selector,
	integer_div_selector: Selector,
}

impl EccUnreducedLadderConfig {
	/// Construct a new config given the selector of child chips
	pub fn new(
		integer_add_selector: Selector, integer_sub_selector: Selector,
		integer_mul_selector: Selector, integer_div_selector: Selector,
	) -> Self {
		Self {
			integer_add_selector,
			integer_sub_selector,
			integer_mul_selector,
			integer_div_selector,
		}
	}
}

/// Chipset structure for the EccUnreducedLadder.
struct EccUnreducedLadderChipset<
	W: FieldExt,
	N: FieldExt,
	const NUM_LIMBS: usize,
	const NUM_BITS: usize,
	P,
> where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	// Assigned point p
	p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
	// Assigned point q
	q: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P>
	EccUnreducedLadderChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	/// Creates a new ecc unreduced ladder chipset.
	pub fn new(
		p: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
		q: AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
	) -> Self {
		Self { p, q }
	}
}

impl<W: FieldExt, N: FieldExt, const NUM_LIMBS: usize, const NUM_BITS: usize, P> Chipset<N>
	for EccUnreducedLadderChipset<W, N, NUM_LIMBS, NUM_BITS, P>
where
	P: RnsParams<W, N, NUM_LIMBS, NUM_BITS>,
{
	type Config = EccUnreducedLadderConfig;
	type Output = AssignedPoint<W, N, NUM_LIMBS, NUM_BITS, P>;

	/// Synthesize the circuit.
	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<N>,
	) -> Result<Self::Output, Error> {
		// numerator = q.y.sub(&p.y)
		let numerator_chip = IntegerSubChip::new(self.q.y, self.p.y.clone());
		let numerator = numerator_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "numerator"),
		)?;

		// denominator = q.x.sub(&p.x)
		let denominator_chip = IntegerSubChip::new(self.q.x.clone(), self.p.x.clone());
		let denominator = denominator_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "denominator"),
		)?;

		// m_zero = numerator.result.div(&denominator.result)
		let m_zero_chip = IntegerDivChip::new(numerator, denominator);
		let m_zero = m_zero_chip.synthesize(
			&common,
			&config.integer_div_selector,
			layouter.namespace(|| "m_zero"),
		)?;

		// m_zero_squared = m_zero.result.mul(&m_zero.result)
		let m_zero_squared_chip = IntegerMulChip::new(m_zero.clone(), m_zero.clone());
		let m_zero_squared = m_zero_squared_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_zero_squared"),
		)?;

		// m_zero_squared_minus_p_x = m_zero_squared.result.sub(&p.x)
		let m_zero_squared_minus_p_x_chip = IntegerSubChip::new(m_zero_squared, self.p.x.clone());
		let m_zero_squared_minus_p_x = m_zero_squared_minus_p_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "m_zero_squared_minus_p_x"),
		)?;

		// x_three = m_zero_squared_minus_p_x.result.sub(&q.x)
		let x_three_chip = IntegerSubChip::new(m_zero_squared_minus_p_x, self.q.x.clone());
		let x_three = x_three_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "x_three"),
		)?;

		// double_p_y = p.y.add(&p.y);
		let double_p_y_chip = IntegerAddChip::new(self.p.y.clone(), self.p.y.clone());
		let double_p_y = double_p_y_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "double_p_y"),
		)?;

		// denom_m_one = x_three.result.sub(&double_p_y.result);
		let denom_m_one_chip = IntegerSubChip::new(x_three.clone(), self.p.x.clone());
		let denom_m_one = denom_m_one_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "denom_m_one"),
		)?;

		// div_res = double_p_y.result.div(&denom_m_one)
		let div_res_chip = IntegerDivChip::new(double_p_y, denom_m_one);
		let div_res = div_res_chip.synthesize(
			&common,
			&config.integer_div_selector,
			layouter.namespace(|| "div_res"),
		)?;

		// m_one = m_zero.result.add(&div_res.result);
		let m_one_chip = IntegerAddChip::new(m_zero, div_res);
		let m_one = m_one_chip.synthesize(
			&common,
			&config.integer_add_selector,
			layouter.namespace(|| "m_one"),
		)?;

		// m_one_squared = m_one.result.mul(&m_one.result);
		let m_one_squared_chip = IntegerMulChip::new(m_one.clone(), m_one.clone());
		let m_one_squared = m_one_squared_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_one_squared"),
		)?;

		// m_one_squared_minus_r_x =
		// m_one_squared.result.sub(&x_three.result);
		let m_one_squared_minus_r_x_chip = IntegerSubChip::new(m_one_squared, x_three);
		let m_one_squared_minus_r_x = m_one_squared_minus_r_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "m_one_squared_minus_r_x"),
		)?;

		// r_x = m_one_squared_minus_r_x.result.sub(&p.x);
		let r_x_chip = IntegerSubChip::new(m_one_squared_minus_r_x, self.p.x.clone());
		let r_x = r_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_x"),
		)?;

		// r_x_minus_p_x = r_x.result.sub(&p.x);
		let r_x_minus_p_x_chip = IntegerSubChip::new(r_x.clone(), self.p.x);
		let r_x_minus_p_x = r_x_minus_p_x_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_x_minus_p_x"),
		)?;

		// m_one_times_r_x_minus_p_x =
		// m_one.result.mul(&r_x_minus_p_x.result);
		let m_one_times_r_x_minus_p_x_chip = IntegerMulChip::new(m_one, r_x_minus_p_x);
		let m_one_times_r_x_minus_p_x = m_one_times_r_x_minus_p_x_chip.synthesize(
			&common,
			&config.integer_mul_selector,
			layouter.namespace(|| "m_one_times_r_x_minus_p_x"),
		)?;

		// r_y = m_one_times_r_x_minus_p_x.result.sub(&p.y);
		let r_y_chip = IntegerSubChip::new(m_one_times_r_x_minus_p_x, self.p.y);
		let r_y = r_y_chip.synthesize(
			&common,
			&config.integer_sub_selector,
			layouter.namespace(|| "r_y"),
		)?;
		let r = AssignedPoint::new(r_x, r_y);
		Ok(r)
	}
}

/*
pub fn mul_scalar(
	// Assigns a cell for the r_x.
	exp_x: [AssignedCell<N, N>; NUM_LIMBS],
	// Assigns a cell for the r_y.
	exp_y: [AssignedCell<N, N>; NUM_LIMBS],
	// Reduction witness for exp_x -- make sure exp_x is in the W field before being passed
	exp_x_rw: ReductionWitness<W, N, NUM_LIMBS, NUM_BITS, P>,
	// Reduction witness for exp_y -- make sure exp_y is in the W field before being passed
	exp_y_rw: ReductionWitness<W, N, NUM_LIMBS, NUM_BITS, P>,
	// Assigns a cell for the value.
	value: AssignedCell<N, N>,
	// Constructs an array for the value bits.
	value_bits: [N; 256],
	// Reduction witnesses for mul scalar add operation
	reduction_witnesses_add: [Vec<ReductionWitness<W, N, NUM_LIMBS, NUM_BITS, P>>; 256],
	// Reduction witnesses for mul scalar double operation
	reduction_witnesses_double: [Vec<ReductionWitness<W, N, NUM_LIMBS, NUM_BITS, P>>; 256],
	// Ecc Config
	config: EccConfig<NUM_LIMBS>,
	// Layouter
	mut layouter: impl Layouter<N>,
) -> Result<
	(
		[AssignedCell<N, N>; NUM_LIMBS],
		[AssignedCell<N, N>; NUM_LIMBS],
	),
	Error,
> {
	// Check that `value_bits` are decomposed from `value`
	// for i in 0..value_bits.len() {
	//    if value_bits[i] == 1 {
	//        add selector - row i
	//    }
	//    double selector - row i
	// }
	let bits2num = Bits2NumChip::new(value.clone(), value_bits.to_vec());
	let bits = bits2num.synthesize(
		&config.common,
		&config.bits2num_selector,
		layouter.namespace(|| "bits2num"),
	)?;
	let mut exp_x = IntegerChip::reduce(
		exp_x,
		exp_x_rw,
		config.integer.clone(),
		layouter.namespace(|| "reduce_exp_x"),
	)?;
	let mut exp_y = IntegerChip::reduce(
		exp_y,
		exp_y_rw,
		config.integer.clone(),
		layouter.namespace(|| "reduce_exp_y"),
	)?;
	let mut exps = Vec::new();
	for i in 0..bits.len() {
		(exp_x, exp_y) = Self::double_unreduced(
			exp_x.clone(),
			exp_y.clone(),
			reduction_witnesses_double[i].clone(),
			config.clone(),
			layouter.namespace(|| "doubling"),
		)?;
		exps.push((exp_x.clone(), exp_y.clone()));
	}
	// Find first positive bit
	let first_bit = Self::find_first_positive_bit(value_bits);
	let mut r_x = exps[first_bit].0.clone();
	let mut r_y = exps[first_bit].1.clone();
	let mut flag = true;

			for i in (first_bit + 1)..bits.len() {
				// Here we pass this checks because we assigned(exp_x, exp_y) to (r_x,
				// r_y) and we already constraint them when we calculate double operation. After
				// we hit second positive bit we start to check addition constraints as well.
				if (value_bits[i] == N::zero()) && flag {
					continue;
				} else {
					flag = false;
					let (new_r_x, new_r_y) = Self::add_unreduced(
						r_x.clone(),
						r_y.clone(),
						exps[i].0.clone(),
						exps[i].1.clone(),
						reduction_witnesses_add[i].clone(),
						config.clone(),
						layouter.namespace(|| "add"),
					)?;

	for i in (first_bit + 1)..bits.len() {
		let (new_r_x, new_r_y) = Self::add_unreduced(
			r_x.clone(),
			r_y.clone(),
			exps[i].0.clone(),
			exps[i].1.clone(),
			reduction_witnesses_add[i].clone(),
			config.clone(),
			layouter.namespace(|| "add"),
		)?;
		for j in 0..NUM_LIMBS {
			// r_x
			let select = SelectChip::new(bits[i].clone(), new_r_x[j].clone(), r_x[j].clone());
			r_x[j] = select.synthesize(
				&config.common,
				&config.select_selector,
				layouter.namespace(|| format!("select_r_x_{}", j)),
			)?;

			// r_y
			let select = SelectChip::new(bits[i].clone(), new_r_y[j].clone(), r_y[j].clone());
			r_y[j] = select.synthesize(
				&config.common,
				&config.select_selector,
				layouter.namespace(|| format!("select_r_y_{}", j)),
			)?;
		}
	}
	Ok((r_x, r_y))
}
fn find_first_positive_bit(input: [N; 256]) -> usize {
	let mut counter = 0;
	for i in 0..256 {
		if input[i] == N::one() {
			break;
		}
		counter += 1;
	}
	counter
}
*/

#[cfg(test)]
mod test {
	use super::{
		AssignedPoint, EccAddChipset, EccAddConfig, EccDoubleChipset, EccDoubleConfig,
		EccUnreducedLadderChipset, EccUnreducedLadderConfig,
	};
	use crate::{
		ecc::native::EcPoint,
		integer::{
			native::Integer, rns::Bn256_4_68, AssignedInteger, IntegerAddChip, IntegerDivChip,
			IntegerMulChip, IntegerReduceChip, IntegerSubChip,
		},
		Chip, Chipset, CommonConfig, RegionCtx,
	};
	use halo2::{
		circuit::{AssignedCell, Layouter, Region, SimpleFloorPlanner, Value},
		dev::MockProver,
		halo2curves::bn256::{Fq, Fr},
		plonk::{Circuit, ConstraintSystem, Error},
	};
	use num_bigint::BigUint;
	use std::str::FromStr;

	type W = Fq;
	type N = Fr;
	const NUM_LIMBS: usize = 4;
	const NUM_BITS: usize = 68;
	type P = Bn256_4_68;

	#[derive(Clone)]
	enum Gadgets {
		Add,
		Double,
		Ladder,
		//Mul,
	}

	#[derive(Clone, Debug)]
	struct TestConfig {
		common: CommonConfig,
		ecc_add: EccAddConfig,
		ecc_double: EccDoubleConfig,
		ecc_ladder: EccUnreducedLadderConfig,
	}

	#[derive(Clone)]
	struct TestCircuit {
		p: EcPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
		q: Option<EcPoint<W, N, NUM_LIMBS, NUM_BITS, P>>,
		value: Option<N>,
		value_bits: Option<[N; 256]>,
		gadget: Gadgets,
	}

	impl TestCircuit {
		fn new(
			p: EcPoint<W, N, NUM_LIMBS, NUM_BITS, P>,
			q: Option<EcPoint<W, N, NUM_LIMBS, NUM_BITS, P>>, value: Option<N>,
			value_bits: Option<[N; 256]>, gadget: Gadgets,
		) -> Self {
			Self { p, q, value, value_bits, gadget }
		}
	}

	impl Circuit<N> for TestCircuit {
		type Config = TestConfig;
		type FloorPlanner = SimpleFloorPlanner;

		fn without_witnesses(&self) -> Self {
			self.clone()
		}

		fn configure(meta: &mut ConstraintSystem<N>) -> TestConfig {
			let common = CommonConfig::new(meta);

			let integer_reduce_selector =
				IntegerReduceChip::<W, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_add_selector =
				IntegerAddChip::<W, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_sub_selector =
				IntegerSubChip::<W, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_mul_selector =
				IntegerMulChip::<W, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);
			let integer_div_selector =
				IntegerDivChip::<W, N, NUM_LIMBS, NUM_BITS, P>::configure(&common, meta);

			let ecc_add = EccAddConfig::new(
				integer_reduce_selector, integer_sub_selector, integer_mul_selector,
				integer_div_selector,
			);

			let ecc_double = EccDoubleConfig::new(
				integer_reduce_selector, integer_add_selector, integer_sub_selector,
				integer_mul_selector, integer_div_selector,
			);

			let ecc_ladder = EccUnreducedLadderConfig::new(
				integer_add_selector, integer_sub_selector, integer_mul_selector,
				integer_div_selector,
			);

			TestConfig { common, ecc_add, ecc_double, ecc_ladder }
		}

		fn synthesize(
			&self, config: TestConfig, mut layouter: impl Layouter<N>,
		) -> Result<(), Error> {
			/*
			let value = layouter.assign_region(
				|| "scalar_mul_values",
				|mut region: Region<'_, N>| {
					let value = region.assign_advice(
						|| "value",
						config.common.advice[0],
						0,
						|| Value::known(self.value.unwrap_or(N::zero())),
					)?;

					Ok(value)
				},
			)?;
			*/

			let (p_x_limbs, p_y_limbs) = layouter.assign_region(
				|| "p_temp",
				|region: Region<'_, N>| {
					let mut ctx = RegionCtx::new(region, 0);
					let mut x_limbs: [Option<AssignedCell<N, N>>; NUM_LIMBS] =
						[(); NUM_LIMBS].map(|_| None);
					let mut y_limbs: [Option<AssignedCell<N, N>>; NUM_LIMBS] =
						[(); NUM_LIMBS].map(|_| None);
					for i in 0..NUM_LIMBS {
						let x = ctx.assign_advice(
							config.common.advice[0],
							Value::known(self.p.x.limbs[i]),
						)?;

						let y = ctx.assign_advice(
							config.common.advice[1],
							Value::known(self.p.y.limbs[i]),
						)?;
						x_limbs[i] = Some(x);
						y_limbs[i] = Some(y);
						ctx.next();
					}

					Ok((x_limbs.map(|x| x.unwrap()), y_limbs.map(|y| y.unwrap())))
				},
			)?;

			let (q_x_limbs, q_y_limbs) = layouter.assign_region(
				|| "q_temp",
				|mut region: Region<'_, N>| {
					let mut x_limbs: [Option<AssignedCell<N, N>>; NUM_LIMBS] =
						[(); NUM_LIMBS].map(|_| None);
					let mut y_limbs: [Option<AssignedCell<N, N>>; NUM_LIMBS] =
						[(); NUM_LIMBS].map(|_| None);
					for i in 0..NUM_LIMBS {
						let x = region.assign_advice(
							|| "temp_x",
							config.common.advice[0],
							i,
							|| {
								Value::known(
									self.q.clone().map(|p| p.x.limbs[i]).unwrap_or(N::zero()),
								)
							},
						)?;
						let y = region.assign_advice(
							|| "temp_y",
							config.common.advice[0],
							i + NUM_LIMBS,
							|| {
								Value::known(
									self.q.clone().map(|p| p.y.limbs[i]).unwrap_or(N::zero()),
								)
							},
						)?;

						x_limbs[i] = Some(x);
						y_limbs[i] = Some(y);
					}

					Ok((x_limbs.map(|x| x.unwrap()), y_limbs.map(|x| x.unwrap())))
				},
			)?;

			let p_x_int = AssignedInteger::new(self.p.x.clone(), p_x_limbs);
			let p_y_int = AssignedInteger::new(self.p.y.clone(), p_y_limbs);

			let p = AssignedPoint::new(p_x_int, p_y_int);

			let result;
			match self.gadget {
				Gadgets::Add => {
					let q_x_int = AssignedInteger::new(self.q.clone().unwrap().x, q_x_limbs);
					let q_y_int = AssignedInteger::new(self.q.clone().unwrap().y, q_y_limbs);
					let q = AssignedPoint::new(q_x_int, q_y_int);

					let chip = EccAddChipset::new(p, q);
					result = Some(chip.synthesize(
						&config.common,
						&config.ecc_add,
						layouter.namespace(|| "ecc_add"),
					)?);
				},

				Gadgets::Double => {
					let chip = EccDoubleChipset::new(p);
					result = Some(chip.synthesize(
						&config.common,
						&config.ecc_double,
						layouter.namespace(|| "ecc_double"),
					)?);
				},

				Gadgets::Ladder => {
					let q_x_int = AssignedInteger::new(self.q.clone().unwrap().x, q_x_limbs);
					let q_y_int = AssignedInteger::new(self.q.clone().unwrap().y, q_y_limbs);
					let q = AssignedPoint::new(q_x_int, q_y_int);

					let chip = EccUnreducedLadderChipset::new(p, q);
					result = Some(chip.synthesize(
						&config.common,
						&config.ecc_ladder,
						layouter.namespace(|| "ecc_ladder"),
					)?);
				},
				/*
				Gadgets::Mul => EccChip::mul_scalar(
					p_x_limbs,
					p_y_limbs,
					self.p_x_rw.clone(),
					self.p_y_rw.clone(),
					value,
					self.value_bits.unwrap(),
					self.reduction_witnesses_add.clone().unwrap(),
					self.reduction_witnesses_double.clone().unwrap(),
					config.ecc.clone(),
					layouter.namespace(|| "scalar_mul"),
				)?,
				*/
			};

			for i in 0..NUM_LIMBS {
				layouter.constrain_instance(
					result.clone().unwrap().x.integer_limbs[i].cell(),
					config.common.instance,
					i,
				)?;
				layouter.constrain_instance(
					result.clone().unwrap().y.integer_limbs[i].cell(),
					config.common.instance,
					i + NUM_LIMBS,
				)?;
			}
			Ok(())
		}
	}

	#[test]
	fn should_add_two_points() {
		// Testing add.
		let a_big = BigUint::from_str("23423423525345345").unwrap();
		let b_big = BigUint::from_str("65464575675").unwrap();
		let c_big = BigUint::from_str("23423423423425345647567567568").unwrap();
		let a = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a_big);
		let b = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(b_big);
		let c = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(c_big);
		let p_point = EcPoint::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a.clone(), b.clone());
		let q_point = EcPoint::<W, N, NUM_LIMBS, NUM_BITS, P>::new(b.clone(), c.clone());

		let res = p_point.add(&q_point);
		let test_chip = TestCircuit::new(p_point, Some(q_point), None, None, Gadgets::Add);

		let k = 7;
		let mut p_ins = Vec::new();
		p_ins.extend(res.x.limbs);
		p_ins.extend(res.y.limbs);
		let prover = MockProver::run(k, &test_chip, vec![p_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn should_double_a_point() {
		// Testing double.
		let a_big = BigUint::from_str("23423423525345345").unwrap();
		let b_big = BigUint::from_str("65464575675").unwrap();
		let a = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a_big);
		let b = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(b_big);
		let p_point = EcPoint::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a.clone(), b.clone());

		let res = p_point.double();
		let test_chip = TestCircuit::new(p_point, None, None, None, Gadgets::Double);

		let k = 7;
		let mut p_ins = Vec::new();
		p_ins.extend(res.x.limbs);
		p_ins.extend(res.y.limbs);
		let prover = MockProver::run(k, &test_chip, vec![p_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn should_ladder_points() {
		// Testing ladder.
		let a_big = BigUint::from_str("23423423525345345").unwrap();
		let b_big = BigUint::from_str("65464575675").unwrap();
		let c_big = BigUint::from_str("23423423423425345647567567568").unwrap();
		let a = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a_big);
		let b = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(b_big);
		let c = Integer::<W, N, NUM_LIMBS, NUM_BITS, P>::new(c_big);
		let p_point = EcPoint::<W, N, NUM_LIMBS, NUM_BITS, P>::new(a.clone(), c.clone());
		let q_point = EcPoint::<W, N, NUM_LIMBS, NUM_BITS, P>::new(b.clone(), c.clone());

		let res = p_point.ladder(&q_point);
		let test_chip = TestCircuit::new(p_point, Some(q_point), None, None, Gadgets::Ladder);

		let k = 7;
		let mut p_ins = Vec::new();
		p_ins.extend(res.x.limbs);
		p_ins.extend(res.y.limbs);
		let prover = MockProver::run(k, &test_chip, vec![p_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	/*
	#[test]
	#[ignore = "Mul scalar broken"]
	fn should_mul_with_scalar() {
		// Testing scalar multiplication.
		let rng = &mut thread_rng();
		let scalar = Fr::from_u128(30);

		let zero = Integer::<Fq, Fr, 4, 68, Bn256_4_68>::zero();
		let a_big = BigUint::from_str("2342342453654645641233").unwrap();
		let b_big = BigUint::from_str("1231231231234235346457675685645454").unwrap();
		let a = Integer::<Fq, Fr, 4, 68, Bn256_4_68>::new(a_big);
		let b = Integer::<Fq, Fr, 4, 68, Bn256_4_68>::new(b_big);
		let p_point = EcPoint::<Fq, Fr, 4, 68, Bn256_4_68>::new(a.clone(), b.clone());
		let p_point = p_point.double();
		let rw_p_x = p_point.x.add(&zero);
		let rw_p_y = p_point.y.add(&zero);

		let bits = scalar.to_bytes().map(|byte| {
			let mut byte_bits = [false; 8];
			for i in (0..8).rev() {
				byte_bits[i] = (byte >> i) & 1u8 != 0
			}
			byte_bits
		});
		let mut bits_fr = [Fr::zero(); 256];
		for i in 0..256 {
			bits_fr[i] = Fr::from_u128(bits.flatten()[i].into())
		}

		let res = p_point.mul_scalar(scalar.to_bytes());
		let test_chip = TestCircuit::<Fq, Fr, 4, 68, Bn256_4_68>::new(
			p_point.clone(),
			rw_p_x.clone(),
			rw_p_y.clone(),
			Some(p_point.clone()),
			Some(rw_p_x.clone()),
			Some(rw_p_y.clone()),
			None,
			Some(res.1.clone()),
			Some(res.2.clone()),
			Some(scalar.clone()),
			Some(bits_fr),
			Gadgets::Mul,
		);
		let k = 13;
		let mut p_ins = Vec::new();
		p_ins.extend(res.0.x.limbs);
		p_ins.extend(res.0.y.limbs);
		let prover = MockProver::run(k, &test_chip, vec![vec![], p_ins]).unwrap();
		let errs = prover.verify().err().unwrap();
		for err in errs {
			println!("{:?}", err);
		}
		//assert_eq!(prover.verify(), Ok(()));
	}
	*/
}
