use crate::{Chip, Chipset, CommonConfig};
use halo2::{
	arithmetic::FieldExt,
	circuit::{AssignedCell, Layouter, Region, Value},
	plonk::{ConstraintSystem, Error, Expression, Selector},
	poly::Rotation,
};

/// Chip for multiplication operation
pub struct MulChip<F: FieldExt> {
	// Assigns a cell for the x.
	x: AssignedCell<F, F>,
	// Assigns a cell for the y.
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> MulChip<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { x, y }
	}
}

impl<F: FieldExt> Chip<F> for MulChip<F> {
	type Output = AssignedCell<F, F>;

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();
		// Gate for the mul circuit.
		meta.create_gate("mul", |v_cells| {
			let x_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let y_exp = v_cells.query_advice(common.advice[1], Rotation::cur());
			let res_exp = v_cells.query_advice(common.advice[2], Rotation::cur());
			let s_exp = v_cells.query_selector(selector);

			vec![
				// (x * y) - z == 0
				// Example:
				// let x = 3;
				// let y = 2;
				// let z = (x * y);
				// z;
				//
				// z = (3 * 2) = 6 => Checking the constraint (3 * 2) - 6 == 0
				s_exp * ((x_exp * y_exp) - res_exp),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "mul",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;

				let assigned_x = self.x.copy_advice(|| "x", &mut region, common.advice[0], 0)?;
				let assigned_y = self.y.copy_advice(|| "y", &mut region, common.advice[1], 0)?;

				let res = assigned_x.value().cloned() * assigned_y.value();

				let res_assigned = region.assign_advice(|| "res", common.advice[2], 0, || res)?;

				Ok(res_assigned)
			},
		)
	}
}

/// Chip for constraining a value to be a boolean
pub struct ConstrainBoolChip<F: FieldExt> {
	x: AssignedCell<F, F>,
}

impl<F: FieldExt> ConstrainBoolChip<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>) -> Self {
		Self { x }
	}
}

impl<F: FieldExt> Chip<F> for ConstrainBoolChip<F> {
	type Output = ();

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();

		// Gate for the is_bool circuit.
		meta.create_gate("constrain_bool", |v_cells| {
			let one = Expression::Constant(F::one());
			let x_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let s_exp = v_cells.query_selector(selector);

			vec![
				// (1 - x) * x == 0
				// Only two valid examples exist for a boolean gate.
				// Circuit working only on current rotation cells.
				// First example:
				// If x = 1,
				// (1 - 1) * 1 == 0 => Checking the constraint 0 * 1 == 0
				// Second example:
				// If x = 0,
				// (1 - 0) * 0 == 0 => Checking the constraint 1 * 0 == 0
				s_exp * ((one - x_exp.clone()) * x_exp),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "constrain_boolean",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;
				self.x.copy_advice(|| "x", &mut region, common.advice[0], 0)?;

				Ok(())
			},
		)
	}
}

/// Chip for checking if a number is zero
pub struct IsZeroChip<F: FieldExt> {
	x: AssignedCell<F, F>,
}

impl<F: FieldExt> IsZeroChip<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>) -> Self {
		Self { x }
	}
}

impl<F: FieldExt> Chip<F> for IsZeroChip<F> {
	type Output = AssignedCell<F, F>;

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();

		meta.create_gate("is_zero", |v_cells| {
			let one = Expression::Constant(F::one());
			let x_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let x_inv_exp = v_cells.query_advice(common.advice[1], Rotation::cur());
			let b_exp = v_cells.query_advice(common.advice[2], Rotation::cur());
			let sel_exp = v_cells.query_selector(selector);

			vec![
				// x * b == 0
				// Checking this constraint to be sure
				// that one of the variable is equal to 0.
				// b is the boolean and desired output is (x == 0)
				sel_exp.clone() * (x_exp.clone() * b_exp.clone()),
				// x * x_inv + b - 1 == 0
				// Example 1:
				// If x = 1 => x_inv = 1,
				// (1 * 1 + b - 1) == 0
				// In this case, b must be equal to 0.
				// Example 2:
				// If b = 1,
				// (x * x_inv + 1 - 1) == 0 => (x * x_inv) must be equal to 0.
				// Which is only can be obtainable by x = 0.
				sel_exp * (x_exp * x_inv_exp + b_exp - one),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "is_zero",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;

				let one = Value::known(F::one());
				let x_inv = self.x.value().and_then(|val| {
					let val_opt: Option<F> = val.invert().into();
					Value::known(val_opt.unwrap_or(F::one()))
				});
				// In the circuit here, if x = 0, b will be assigned to the value 1.
				// If x = 1, means x_inv = 1 as well, b will be assigned to the value 0.
				let b = one - self.x.value().cloned() * x_inv;

				self.x.copy_advice(|| "x", &mut region, common.advice[0], 0)?;
				region.assign_advice(|| "x_inv", common.advice[1], 0, || x_inv)?;
				let assigned_b = region.assign_advice(|| "b", common.advice[2], 0, || b)?;

				Ok(assigned_b)
			},
		)
	}
}

/// A chip for add operation
pub struct AddChip<F: FieldExt> {
	x: AssignedCell<F, F>,
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> AddChip<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { x, y }
	}
}

impl<F: FieldExt> Chip<F> for AddChip<F> {
	type Output = AssignedCell<F, F>;

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();

		meta.create_gate("add", |v_cells| {
			let x_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let y_exp = v_cells.query_advice(common.advice[1], Rotation::cur());
			let z_exp = v_cells.query_advice(common.advice[2], Rotation::cur());
			let s_exp = v_cells.query_selector(selector);

			vec![
				// (x + y) - z == 0
				// Example:
				// let x = 3;
				// let y = 2;
				// let z = (x + y);
				// z;
				//
				// z = (3 + 2) = 5 => Checking the constraint (3 + 2) - 5 == 0
				s_exp * ((x_exp + y_exp) - z_exp),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "add",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;
				let assigned_x = self.x.copy_advice(|| "x", &mut region, common.advice[0], 0)?;
				let assigned_y = self.y.copy_advice(|| "y", &mut region, common.advice[1], 0)?;

				let out = assigned_x.value().cloned() + assigned_y.value();

				let out_assigned = region.assign_advice(|| "out", common.advice[2], 0, || out)?;

				Ok(out_assigned)
			},
		)
	}
}

/// A chip for subtract operation
pub struct SubChip<F: FieldExt> {
	x: AssignedCell<F, F>,
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> SubChip<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { x, y }
	}
}

impl<F: FieldExt> Chip<F> for SubChip<F> {
	type Output = AssignedCell<F, F>;

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();

		meta.create_gate("sub", |v_cells| {
			let lhs_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let rhs_exp = v_cells.query_advice(common.advice[1], Rotation::cur());
			let out_exp = v_cells.query_advice(common.advice[2], Rotation::cur());
			let s_exp = v_cells.query_selector(selector);

			vec![
				// (x + y) - z == 0
				// Example:
				// let y = 123;
				// let z = 123;
				// let x = (y - z);
				// x;
				//
				// x = (123 - 123) = 0 => Checking the constraint (0 + 123) - 123 == 0
				s_exp * ((out_exp + rhs_exp) - lhs_exp),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "sub",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;
				let assigned_lhs =
					self.x.copy_advice(|| "lhs", &mut region, common.advice[0], 0)?;
				let assigned_rhs =
					self.y.copy_advice(|| "rhs", &mut region, common.advice[1], 0)?;

				let out = assigned_lhs.value().cloned() - assigned_rhs.value();

				let assigned_out = region.assign_advice(|| "out", common.advice[2], 0, || out)?;
				Ok(assigned_out)
			},
		)
	}
}

/// A chip for selecting a value based on a bit
/// This chip does NOT checks the validity of the bit
pub struct SelectChip<F: FieldExt> {
	bit: AssignedCell<F, F>,
	x: AssignedCell<F, F>,
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> SelectChip<F> {
	/// Construct a new chip
	pub fn new(bit: AssignedCell<F, F>, x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { bit, x, y }
	}
}

impl<F: FieldExt> Chip<F> for SelectChip<F> {
	type Output = AssignedCell<F, F>;

	fn configure(common: &CommonConfig, meta: &mut ConstraintSystem<F>) -> Selector {
		let selector = meta.selector();

		meta.create_gate("select", |v_cells| {
			let bit_exp = v_cells.query_advice(common.advice[0], Rotation::cur());
			let x_exp = v_cells.query_advice(common.advice[1], Rotation::cur());
			let y_exp = v_cells.query_advice(common.advice[2], Rotation::cur());
			let res_exp = v_cells.query_advice(common.advice[3], Rotation::cur());
			let s_exp = v_cells.query_selector(selector);

			vec![
				// bit * (x - y) - (z - y)
				// Example 1:
				// bit = 1
				// z will carry the same value with x when bit == 1. (x == z)
				// x = 5
				// y = 3
				// z = 5
				// 1 * (x - y) - (z - y) = 1 * (5 - 3) - (5 - 3) = 0
				// Example 2:
				// bit = 0
				// z will carry the same value with y when bit == 0. (y == z)
				// x = 5
				// y = 3
				// z = 3
				// 0 * (x - y) - (z - y) = 0 * (5 - 3) - (3 - 3) = 0
				s_exp * (bit_exp.clone() * (x_exp - y_exp.clone()) - (res_exp - y_exp)),
			]
		});

		selector
	}

	fn synthesize(
		self, common: &CommonConfig, selector: &Selector, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		layouter.assign_region(
			|| "select",
			|mut region: Region<'_, F>| {
				selector.enable(&mut region, 0)?;

				let assigned_bit =
					self.bit.copy_advice(|| "bit", &mut region, common.advice[0], 0)?;
				let assigned_x = self.x.copy_advice(|| "x", &mut region, common.advice[1], 0)?;
				let assigned_y = self.y.copy_advice(|| "y", &mut region, common.advice[2], 0)?;

				// Conditional control checks the bit. Is it zero or not?
				// If yes returns the y value, else x.
				let res = assigned_bit.value().and_then(|bit_f| {
					if bool::from(bit_f.is_zero()) {
						assigned_y.value().cloned()
					} else {
						assigned_x.value().cloned()
					}
				});

				let assigned_res = region.assign_advice(|| "res", common.advice[3], 0, || res)?;

				Ok(assigned_res)
			},
		)
	}
}

#[derive(Clone)]
/// Selector for IsEqualChipset
pub struct IsEqualConfig {
	sub_selector: Selector,
	is_zero_selector: Selector,
}

impl IsEqualConfig {
	/// Constructs a new config given the selectors
	pub fn new(sub_selector: Selector, is_zero_selector: Selector) -> Self {
		Self { sub_selector, is_zero_selector }
	}
}

/// A chip for checking equality
pub struct IsEqualChipset<F: FieldExt> {
	x: AssignedCell<F, F>,
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> IsEqualChipset<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { x, y }
	}
}

impl<F: FieldExt> Chipset<F> for IsEqualChipset<F> {
	type Config = IsEqualConfig;
	type Output = AssignedCell<F, F>;

	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		let sub_chipset = SubChip::new(self.x, self.y);
		let res =
			sub_chipset.synthesize(common, &config.sub_selector, layouter.namespace(|| "diff"))?;

		let is_zero_chip = IsZeroChip::new(res);
		let is_zero = is_zero_chip.synthesize(
			common,
			&config.is_zero_selector,
			layouter.namespace(|| "is_zero"),
		)?;

		Ok(is_zero)
	}
}

#[derive(Clone)]
/// Selectors for AndChipset
pub struct AndConfig {
	bool_selector: Selector,
	mul_selector: Selector,
}

impl AndConfig {
	/// Construct the config given the selectors
	pub fn new(mul_selector: Selector, bool_selector: Selector) -> Self {
		Self { mul_selector, bool_selector }
	}
}

/// A chipset for bitwise AND operation.
/// Requires that values are bits
pub struct AndChipset<F: FieldExt> {
	// Assigns a cell for the x.
	x: AssignedCell<F, F>,
	// Assigns a cell for the y.
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> AndChipset<F> {
	/// Construct a new chip
	pub fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>) -> Self {
		Self { x, y }
	}
}

impl<F: FieldExt> Chipset<F> for AndChipset<F> {
	type Config = AndConfig;
	type Output = AssignedCell<F, F>;

	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		let bch_x = ConstrainBoolChip::new(self.x.clone());
		bch_x.synthesize(
			common,
			&config.bool_selector,
			layouter.namespace(|| "bool_constraint_x"),
		)?;
		let bch_y = ConstrainBoolChip::new(self.y.clone());
		bch_y.synthesize(
			common,
			&config.bool_selector,
			layouter.namespace(|| "bool_constraint_y"),
		)?;

		let mul_chip = MulChip::new(self.x, self.y);
		let and_res = mul_chip.synthesize(
			common,
			&config.mul_selector,
			layouter.namespace(|| "mul_boolean"),
		)?;

		Ok(and_res)
	}
}

#[derive(Clone)]
/// Selectors for the StrictSelectChipset
pub struct StrictSelectConfig {
	bool_selector: Selector,
	select_selector: Selector,
}

/// A chip for selecting a value based on a bit
/// This chipset checks the validity on the bit
pub struct StrictSelectChipset<F: FieldExt> {
	bit: AssignedCell<F, F>,
	x: AssignedCell<F, F>,
	y: AssignedCell<F, F>,
}

impl<F: FieldExt> StrictSelectChipset<F> {
	/// Construct new chip
	fn new(x: AssignedCell<F, F>, y: AssignedCell<F, F>, bit: AssignedCell<F, F>) -> Self {
		Self { x, y, bit }
	}
}

impl<F: FieldExt> Chipset<F> for StrictSelectChipset<F> {
	type Config = StrictSelectConfig;
	type Output = AssignedCell<F, F>;

	fn synthesize(
		self, common: &CommonConfig, config: &Self::Config, mut layouter: impl Layouter<F>,
	) -> Result<Self::Output, Error> {
		let bool_chip = ConstrainBoolChip::new(self.bit.clone());
		bool_chip.synthesize(
			common,
			&config.bool_selector,
			layouter.namespace(|| "constrain_bit"),
		)?;

		let select_chip = SelectChip::new(self.x, self.y, self.bit);
		let res = select_chip.synthesize(
			common,
			&config.select_selector,
			layouter.namespace(|| "select"),
		)?;

		Ok(res)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		utils::{generate_params, prove_and_verify},
		Chip, Chipset, CommonChip,
	};
	use halo2::{
		circuit::{SimpleFloorPlanner, Value},
		dev::MockProver,
		halo2curves::bn256::{Bn256, Fr},
		plonk::Circuit,
	};

	#[derive(Clone)]
	enum Gadgets {
		And,
		IsBool,
		IsEqual,
		IsZero,
		Mul,
		Select,
	}

	#[derive(Clone)]
	struct TestConfig {
		common: CommonConfig,
		is_equal: IsEqualConfig,
		and: AndConfig,
		mul_selector: Selector,
		is_bool_selector: Selector,
		is_zero_selector: Selector,
		select_selector: Selector,
	}

	#[derive(Clone)]
	struct TestCircuit<F: FieldExt, const N: usize> {
		inputs: [F; N],
		gadget: Gadgets,
	}

	impl<F: FieldExt, const N: usize> TestCircuit<F, N> {
		fn new(inputs: [F; N], gadget: Gadgets) -> Self {
			Self { inputs, gadget }
		}
	}

	impl<F: FieldExt, const N: usize> Circuit<F> for TestCircuit<F, N> {
		type Config = TestConfig;
		type FloorPlanner = SimpleFloorPlanner;

		fn without_witnesses(&self) -> Self {
			self.clone()
		}

		fn configure(meta: &mut ConstraintSystem<F>) -> TestConfig {
			let common = CommonChip::configure(meta);
			let mul_selector = MulChip::configure(&common, meta);
			let sub_selector = SubChip::configure(&common, meta);
			let is_bool_selector = ConstrainBoolChip::configure(&common, meta);
			let is_zero_selector = IsZeroChip::configure(&common, meta);
			let select_selector = SelectChip::configure(&common, meta);

			let and = AndConfig::new(mul_selector, is_bool_selector);
			let is_equal = IsEqualConfig::new(sub_selector, is_zero_selector);

			TestConfig {
				common,
				is_equal,
				and,
				mul_selector,
				is_bool_selector,
				is_zero_selector,
				select_selector,
			}
		}

		fn synthesize(
			&self, config: TestConfig, mut layouter: impl Layouter<F>,
		) -> Result<(), Error> {
			let mut items = Vec::new();
			for i in 0..N {
				items.push(layouter.assign_region(
					|| "temp",
					|mut region: Region<'_, F>| {
						let x = region.assign_advice(
							|| "temp_inputs",
							config.common.advice[0],
							i,
							|| Value::known(self.inputs[i]),
						)?;
						Ok(x)
					},
				)?);
			}

			match self.gadget {
				Gadgets::And => {
					let and_chip = AndChipset::new(items[0].clone(), items[1].clone());
					let and = and_chip.synthesize(
						&config.common,
						&config.and,
						layouter.namespace(|| "and"),
					)?;
					layouter.constrain_instance(and.cell(), config.common.instance, 0)?;
				},
				Gadgets::IsBool => {
					let is_bool_chip = ConstrainBoolChip::new(items[0].clone());
					is_bool_chip.synthesize(
						&config.common,
						&config.is_bool_selector,
						layouter.namespace(|| "is_bool"),
					)?;
				},
				Gadgets::IsEqual => {
					let is_equal_chip = IsEqualChipset::new(items[0].clone(), items[1].clone());
					let is_equal = is_equal_chip.synthesize(
						&config.common,
						&config.is_equal,
						layouter.namespace(|| "is_equal"),
					)?;
					layouter.constrain_instance(is_equal.cell(), config.common.instance, 0)?;
				},
				Gadgets::IsZero => {
					let is_zero_chip = IsZeroChip::new(items[0].clone());
					let is_zero = is_zero_chip.synthesize(
						&config.common,
						&config.is_zero_selector,
						layouter.namespace(|| "is_zero"),
					)?;
					layouter.constrain_instance(is_zero.cell(), config.common.instance, 0)?;
				},
				Gadgets::Mul => {
					let mul_chip = MulChip::new(items[0].clone(), items[1].clone());
					let mul = mul_chip.synthesize(
						&config.common,
						&config.mul_selector,
						layouter.namespace(|| "mul"),
					)?;
					layouter.constrain_instance(mul.cell(), config.common.instance, 0)?;
				},
				Gadgets::Select => {
					let select_chip =
						SelectChip::new(items[0].clone(), items[1].clone(), items[2].clone());
					let select = select_chip.synthesize(
						&config.common,
						&config.select_selector,
						layouter.namespace(|| "select"),
					)?;
					layouter.constrain_instance(select.cell(), config.common.instance, 0)?;
				},
			}

			Ok(())
		}
	}

	// TEST CASES FOR THE AND CIRCUIT
	#[test]
	fn test_and_x1_y1() {
		// Testing x = 1 and y = 1.
		let test_chip = TestCircuit::new([Fr::from(1), Fr::from(1)], Gadgets::And);

		let pub_ins = vec![Fr::from(1)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_and_x1_y0() {
		// Testing x = 1 and y = 0.
		let test_chip = TestCircuit::new([Fr::from(1), Fr::from(0)], Gadgets::And);

		let pub_ins = vec![Fr::from(0)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_and_x0_y0() {
		// Testing x = 0 and y = 0.
		let test_chip = TestCircuit::new([Fr::from(0), Fr::from(0)], Gadgets::And);

		let pub_ins = vec![Fr::from(0)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_and_x0_y1() {
		// Testing x = 0 and y = 1.
		let test_chip = TestCircuit::new([Fr::from(0), Fr::from(1)], Gadgets::And);

		let pub_ins = vec![Fr::from(0)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_and_production() {
		let test_chip = TestCircuit::new([Fr::from(1), Fr::from(1)], Gadgets::And);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res =
			prove_and_verify::<Bn256, _, _>(params, test_chip, &[&[Fr::from(1)]], rng).unwrap();

		assert!(res);
	}

	// TEST CASES FOR THE IS_BOOL CIRCUIT
	// In a IsBool test case sending a dummy instance doesn't
	// affect the circuit output because it is not constrained.
	#[test]
	fn test_is_bool_value_zero() {
		// Testing input zero as value.
		let test_chip = TestCircuit::new([Fr::from(0)], Gadgets::IsBool);

		let k = 4;
		let dummy_instance = vec![Fr::zero()];
		let prover = MockProver::run(k, &test_chip, vec![dummy_instance]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_bool_value_one() {
		// Testing input one as value.
		let test_chip = TestCircuit::new([Fr::from(1)], Gadgets::IsBool);

		let k = 4;
		let dummy_instance = vec![Fr::zero()];
		let prover = MockProver::run(k, &test_chip, vec![dummy_instance]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_bool_invalid_value() {
		// Testing input two as value, which is invalid for the boolean circuit.
		let test_chip = TestCircuit::new([Fr::from(2)], Gadgets::IsBool);

		let k = 4;
		let dummy_instance = vec![Fr::zero()];
		let prover = MockProver::run(k, &test_chip, vec![dummy_instance]).unwrap();
		assert!(prover.verify().is_err());
	}

	#[test]
	fn test_is_bool_production() {
		let test_chip = TestCircuit::new([Fr::from(0)], Gadgets::IsBool);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let dummy_instance = vec![Fr::zero()];
		let res =
			prove_and_verify::<Bn256, _, _>(params, test_chip, &[&dummy_instance], rng).unwrap();

		assert!(res);
	}

	// TEST CASES FOR THE IS_EQUAL CIRCUIT
	#[test]
	fn test_is_equal() {
		// Testing equal values.
		let test_chip = TestCircuit::new([Fr::from(123), Fr::from(123)], Gadgets::IsEqual);

		let pub_ins = vec![Fr::one()];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_not_equal() {
		// Testing not equal values.
		let test_chip = TestCircuit::new([Fr::from(123), Fr::from(124)], Gadgets::IsEqual);

		let pub_ins = vec![Fr::zero()];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_equal_production() {
		let test_chip = TestCircuit::new([Fr::from(123), Fr::from(123)], Gadgets::IsEqual);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res = prove_and_verify::<Bn256, _, _>(params, test_chip, &[&[Fr::one()]], rng).unwrap();

		assert!(res);
	}

	// TEST CASES FOR THE IS_ZERO CIRCUIT
	#[test]
	fn test_is_zero() {
		// Testing zero as value.
		let test_chip = TestCircuit::new([Fr::from(0)], Gadgets::IsZero);

		let pub_ins = vec![Fr::one()];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_zero_not() {
		// Testing a non-zero value.
		let test_chip = TestCircuit::new([Fr::from(1)], Gadgets::IsZero);

		let pub_ins = vec![Fr::zero()];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_is_zero_production() {
		let test_chip = TestCircuit::new([Fr::from(0)], Gadgets::IsZero);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res = prove_and_verify::<Bn256, _, _>(params, test_chip, &[&[Fr::one()]], rng).unwrap();

		assert!(res);
	}

	// TEST CASES FOR THE MUL CIRCUIT
	#[test]
	fn test_mul() {
		// Testing x = 5 and y = 2.
		let test_chip = TestCircuit::new([Fr::from(5), Fr::from(2)], Gadgets::Mul);

		let k = 4;
		let pub_ins = vec![Fr::from(10)];
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_mul_y1() {
		// Testing x = 3 and y = 1.
		let test_chip = TestCircuit::new([Fr::from(3), Fr::from(1)], Gadgets::Mul);

		let k = 4;
		let pub_ins = vec![Fr::from(3)];
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_mul_y0() {
		// Testing x = 4 and y = 0.
		let test_chip = TestCircuit::new([Fr::from(4), Fr::from(0)], Gadgets::Mul);

		let k = 4;
		let pub_ins = vec![Fr::from(0)];
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_mul_production() {
		let test_chip = TestCircuit::new([Fr::from(5), Fr::from(2)], Gadgets::Mul);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res =
			prove_and_verify::<Bn256, _, _>(params, test_chip, &[&[Fr::from(10)]], rng).unwrap();

		assert!(res);
	}

	// TEST CASES FOR THE SELECT CIRCUIT
	#[test]
	fn test_select() {
		// Testing bit = 0.
		let test_chip = TestCircuit::new([Fr::from(0), Fr::from(2), Fr::from(3)], Gadgets::Select);

		let pub_ins = vec![Fr::from(3)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_select_one_as_bit() {
		// Testing bit = 1.
		let test_chip = TestCircuit::new([Fr::from(1), Fr::from(7), Fr::from(4)], Gadgets::Select);

		let pub_ins = vec![Fr::from(7)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_select_two_as_bit() {
		// Testing bit = 2. Constraint not satisfied error will return
		// because the bit is not a boolean value.
		let test_chip = TestCircuit::new([Fr::from(2), Fr::from(3), Fr::from(6)], Gadgets::Select);

		let pub_ins = vec![Fr::from(3)];
		let k = 4;
		let prover = MockProver::run(k, &test_chip, vec![pub_ins]).unwrap();
		assert!(prover.verify().is_err());
	}

	#[test]
	fn test_select_production() {
		let test_chip = TestCircuit::new([Fr::from(0), Fr::from(2), Fr::from(3)], Gadgets::Select);

		let k = 4;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res =
			prove_and_verify::<Bn256, _, _>(params, test_chip, &[&[Fr::from(3)]], rng).unwrap();

		assert!(res);
	}
}
