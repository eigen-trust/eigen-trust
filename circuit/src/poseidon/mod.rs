/// Native version of Poseidon
pub mod native;
/// Implementation of a Poseidon sponge
pub mod sponge;

use crate::params::RoundParams;
use halo2::{
	arithmetic::FieldExt,
	circuit::{AssignedCell, Layouter, Region, Value},
	plonk::{Advice, Column, ConstraintSystem, Error, Expression, Fixed, Selector, VirtualCells},
	poly::Rotation,
};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
/// Configuration elements for the circuit are defined here.
pub struct PoseidonConfig<const WIDTH: usize> {
	/// Configures columns for the state.
	state: [Column<Advice>; WIDTH],
	/// Configures columns for the round constants.
	round_constants: [Column<Fixed>; WIDTH],
	/// Configures a fixed boolean value for each row of the circuit.
	full_round_selector: Selector,
	/// Configures a fixed boolean value for each row of the circuit.
	partial_round_selector: Selector,
}

/// Constructs a chip structure for the circuit.
pub struct PoseidonChip<F: FieldExt, const WIDTH: usize, P>
where
	P: RoundParams<F, WIDTH>,
{
	/// Constructs a cell array for the inputs.
	inputs: [AssignedCell<F, F>; WIDTH],
	/// Constructs a phantom data for the parameters.
	_params: PhantomData<P>,
}

impl<F: FieldExt, const WIDTH: usize, P> PoseidonChip<F, WIDTH, P>
where
	P: RoundParams<F, WIDTH>,
{
	/// Create a new chip.
	pub fn new(inputs: [AssignedCell<F, F>; WIDTH]) -> Self {
		PoseidonChip { inputs, _params: PhantomData }
	}

	/// Copy given state variables to the circuit.
	fn copy_state(
		config: &PoseidonConfig<WIDTH>, region: &mut Region<'_, F>, round: usize,
		prev_state: &[AssignedCell<F, F>; WIDTH],
	) -> Result<[AssignedCell<F, F>; WIDTH], Error> {
		let mut state: [Option<AssignedCell<F, F>>; WIDTH] = [(); WIDTH].map(|_| None);
		for i in 0..WIDTH {
			state[i] =
				Some(prev_state[i].copy_advice(|| "state", region, config.state[i], round)?);
		}
		Ok(state.map(|item| item.unwrap()))
	}

	/// Assign relevant constants to the circuit for the given round.
	fn load_round_constants(
		config: &PoseidonConfig<WIDTH>, region: &mut Region<'_, F>, round: usize,
		round_constants: &[F],
	) -> Result<[Value<F>; WIDTH], Error> {
		let mut round_values: [Value<F>; WIDTH] = [(); WIDTH].map(|_| Value::unknown());
		for i in 0..WIDTH {
			round_values[i] = Value::known(round_constants[round * WIDTH + i]);
			region.assign_fixed(
				|| "round_constant",
				config.round_constants[i],
				round,
				|| round_values[i],
			)?;
		}
		Ok(round_values)
	}

	/// Add round constants to the state values
	/// for the AddRoundConstants operation.
	fn apply_round_constants(
		state_cells: &[AssignedCell<F, F>; WIDTH], round_const_values: &[Value<F>; WIDTH],
	) -> [Value<F>; WIDTH] {
		let mut next_state = [Value::unknown(); WIDTH];
		for i in 0..WIDTH {
			let state = &state_cells[i];
			let round_const = &round_const_values[i];
			let sum = *round_const + state.value();
			next_state[i] = sum;
		}
		next_state
	}

	/// Compute MDS matrix for MixLayer operation.
	fn apply_mds(next_state: &[Value<F>; WIDTH]) -> [Value<F>; WIDTH] {
		let mut new_state = [Value::known(F::zero()); WIDTH];
		let mds = P::mds();
		for i in 0..WIDTH {
			for j in 0..WIDTH {
				let mds_ij = &Value::known(mds[i][j]);
				let m_product = next_state[j] * mds_ij;
				new_state[i] = new_state[i] + m_product;
			}
		}
		new_state
	}

	/// Add round constants expression to the state values
	/// expression for the AddRoundConstants operation in the circuit.
	fn apply_round_constants_expr(
		v_cells: &mut VirtualCells<F>, state: &[Column<Advice>; WIDTH],
		round_constants: &[Column<Fixed>; WIDTH],
	) -> [Expression<F>; WIDTH] {
		let mut exprs = [(); WIDTH].map(|_| Expression::Constant(F::zero()));
		for i in 0..WIDTH {
			let curr_state = v_cells.query_advice(state[i], Rotation::cur());
			let round_constant = v_cells.query_fixed(round_constants[i], Rotation::cur());
			exprs[i] = curr_state + round_constant;
		}
		exprs
	}

	/// Compute MDS matrix for MixLayer operation in the circuit.
	fn apply_mds_expr(exprs: &[Expression<F>; WIDTH]) -> [Expression<F>; WIDTH] {
		let mut new_exprs = [(); WIDTH].map(|_| Expression::Constant(F::zero()));
		let mds = P::mds();
		// Mat mul with MDS
		for i in 0..WIDTH {
			for j in 0..WIDTH {
				new_exprs[i] = new_exprs[i].clone() + (exprs[j].clone() * mds[i][j]);
			}
		}
		new_exprs
	}

	/// Configures full_round.
	fn full_round(
		config: &PoseidonConfig<WIDTH>, region: &mut Region<'_, F>, num_rounds: usize,
		round_constants: &[F], prev_state: &[AssignedCell<F, F>; WIDTH],
	) -> Result<[AssignedCell<F, F>; WIDTH], Error> {
		// Assign initial state
		let mut state_cells = Self::copy_state(config, region, 0, prev_state)?;
		for round in 0..num_rounds {
			config.full_round_selector.enable(region, round)?;

			// Assign round constants
			let round_const_values =
				Self::load_round_constants(config, region, round, round_constants)?;

			// 1. step for the TRF.
			// AddRoundConstants step.
			let mut next_state = Self::apply_round_constants(&state_cells, &round_const_values);
			for i in 0..WIDTH {
				// 2. step for the TRF.
				// SubWords step, denoted by S-box.
				next_state[i] = next_state[i].map(|s| P::sbox_f(s));
			}

			// 3. step for the TRF.
			// MixLayer step.
			next_state = Self::apply_mds(&next_state);

			// Assign next state
			for i in 0..WIDTH {
				state_cells[i] = region.assign_advice(
					|| "state",
					config.state[i],
					round + 1,
					|| next_state[i],
				)?;
			}
		}
		Ok(state_cells)
	}

	/// Configures partial_round.
	fn partial_round(
		config: &PoseidonConfig<WIDTH>, region: &mut Region<'_, F>, num_rounds: usize,
		round_constants: &[F], prev_state: &[AssignedCell<F, F>; WIDTH],
	) -> Result<[AssignedCell<F, F>; WIDTH], Error> {
		let mut state_cells = Self::copy_state(config, region, 0, prev_state)?;
		for round in 0..num_rounds {
			config.partial_round_selector.enable(region, round)?;

			// Assign round constants
			let round_const_cells =
				Self::load_round_constants(config, region, round, round_constants)?;

			// 1. step for the TRF.
			// AddRoundConstants step.
			let mut next_state = Self::apply_round_constants(&state_cells, &round_const_cells);
			// 2. step for the TRF.
			// SubWords step, denoted by S-box.
			next_state[0] = next_state[0].map(|x| P::sbox_f(x));

			// 3. step for the TRF.
			// MixLayer step.
			next_state = Self::apply_mds(&next_state);

			// Assign next state
			for i in 0..WIDTH {
				state_cells[i] = region.assign_advice(
					|| "state",
					config.state[i],
					round + 1,
					|| next_state[i],
				)?;
			}
		}
		Ok(state_cells)
	}
}

impl<F: FieldExt, const WIDTH: usize, P> PoseidonChip<F, WIDTH, P>
where
	P: RoundParams<F, WIDTH>,
{
	/// Make the circuit config.
	pub fn configure(meta: &mut ConstraintSystem<F>) -> PoseidonConfig<WIDTH> {
		let state = [(); WIDTH].map(|_| meta.advice_column());
		let round_constants = [(); WIDTH].map(|_| meta.fixed_column());
		let full_round_selector = meta.selector();
		let partial_round_selector = meta.selector();

		state.map(|c| meta.enable_equality(c));
		round_constants.map(|c| meta.enable_equality(c));

		meta.create_gate("full_round", |v_cells| {
			// 1. step for the TRF.
			// AddRoundConstants step.
			let mut exprs = Self::apply_round_constants_expr(v_cells, &state, &round_constants);
			// Applying S-boxes for the full round.
			for i in 0..WIDTH {
				// 2. step for the TRF.
				// SubWords step, denoted by S-box.
				exprs[i] = P::sbox_expr(exprs[i].clone());
			}
			// 3. step for the TRF.
			// MixLayer step.
			exprs = Self::apply_mds_expr(&exprs);

			let s_cells = v_cells.query_selector(full_round_selector);
			// It should be equal to the state in next row
			for i in 0..WIDTH {
				let next_state = v_cells.query_advice(state[i], Rotation::next());
				exprs[i] = s_cells.clone() * (exprs[i].clone() - next_state);
			}
			exprs
		});

		meta.create_gate("partial_round", |v_cells| {
			// 1. step for the TRF.
			// AddRoundConstants step.
			let mut exprs = Self::apply_round_constants_expr(v_cells, &state, &round_constants);
			// Applying single S-box for the partial round.
			// 2. step for the TRF.
			// SubWords step, denoted by S-box.
			exprs[0] = P::sbox_expr(exprs[0].clone());

			// 3. step for the TRF.
			// MixLayer step.
			exprs = Self::apply_mds_expr(&exprs);

			let s_cells = v_cells.query_selector(partial_round_selector);
			// It should be equal to the state in next row
			for i in 0..WIDTH {
				let next_state = v_cells.query_advice(state[i], Rotation::next());
				exprs[i] = s_cells.clone() * (exprs[i].clone() - next_state);
			}

			exprs
		});

		PoseidonConfig { state, round_constants, full_round_selector, partial_round_selector }
	}

	/// Synthesize the circuit.
	pub fn synthesize(
		&self, config: &PoseidonConfig<WIDTH>, mut layouter: impl Layouter<F>,
	) -> Result<[AssignedCell<F, F>; WIDTH], Error> {
		let full_rounds = P::full_rounds();
		let half_full_rounds = full_rounds / 2;
		let partial_rounds = P::partial_rounds();
		let round_constants = P::round_constants();
		let total_count = P::round_constants_count();

		let first_round_end = half_full_rounds * WIDTH;
		let first_round_constants = &round_constants[0..first_round_end];

		let second_round_end = first_round_end + partial_rounds * WIDTH;
		let second_round_constants = &round_constants[first_round_end..second_round_end];

		let third_round_constants = &round_constants[second_round_end..total_count];

		// The Hades Design Strategy for Hashing.
		// Mixing rounds with half-full S-box layers and
		// rounds with partial S-box layers.
		// More detailed explanation for
		// The Round Function (TRF) and Hades:
		// https://eprint.iacr.org/2019/458.pdf#page=5
		let state1 = layouter.assign_region(
			|| "full_rounds_1",
			|mut region: Region<'_, F>| {
				Self::full_round(
					&config, &mut region, half_full_rounds, first_round_constants, &self.inputs,
				)
			},
		)?;

		let state2 = layouter.assign_region(
			|| "partial_rounds",
			|mut region: Region<'_, F>| {
				Self::partial_round(
					&config, &mut region, partial_rounds, second_round_constants, &state1,
				)
			},
		)?;

		let state3 = layouter.assign_region(
			|| "full_rounds_2",
			|mut region: Region<'_, F>| {
				Self::full_round(
					&config, &mut region, half_full_rounds, third_round_constants, &state2,
				)
			},
		)?;

		Ok(state3)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		params::{hex_to_field, poseidon_bn254_5x5::Params},
		utils::{generate_params, prove_and_verify},
	};
	use halo2::{
		circuit::{Layouter, SimpleFloorPlanner},
		dev::MockProver,
		halo2curves::bn256::{Bn256, Fr},
		plonk::{Circuit, Column, ConstraintSystem, Error, Instance},
	};

	type TestPoseidonChip = PoseidonChip<Fr, 5, Params>;

	#[derive(Clone)]
	struct PoseidonTesterConfig {
		poseidon_config: PoseidonConfig<5>,
		results: Column<Instance>,
	}

	#[derive(Clone)]
	struct PoseidonTester {
		inputs: [Value<Fr>; 5],
	}

	impl PoseidonTester {
		fn new(inputs: [Value<Fr>; 5]) -> Self {
			Self { inputs }
		}

		fn load_state(
			config: &PoseidonConfig<5>, region: &mut Region<'_, Fr>, round: usize,
			init_state: [Value<Fr>; 5],
		) -> Result<[AssignedCell<Fr, Fr>; 5], Error> {
			let mut state: [Option<AssignedCell<Fr, Fr>>; 5] = [(); 5].map(|_| None);
			for i in 0..5 {
				state[i] = Some(region.assign_advice(
					|| "state",
					config.state[i],
					round,
					|| init_state[i],
				)?);
			}
			Ok(state.map(|item| item.unwrap()))
		}
	}

	impl Circuit<Fr> for PoseidonTester {
		type Config = PoseidonTesterConfig;
		type FloorPlanner = SimpleFloorPlanner;

		fn without_witnesses(&self) -> Self {
			Self { inputs: [Value::unknown(); 5] }
		}

		fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
			let poseidon_config = TestPoseidonChip::configure(meta);
			let results = meta.instance_column();

			meta.enable_equality(results);

			Self::Config { poseidon_config, results }
		}

		fn synthesize(
			&self, config: Self::Config, mut layouter: impl Layouter<Fr>,
		) -> Result<(), Error> {
			let init_state = layouter.assign_region(
				|| "load_state",
				|mut region: Region<'_, Fr>| {
					Self::load_state(&config.poseidon_config, &mut region, 0, self.inputs)
				},
			)?;

			let poseidon = TestPoseidonChip::new(init_state);
			let result_state =
				poseidon.synthesize(&config.poseidon_config, layouter.namespace(|| "poseidon"))?;
			for i in 0..5 {
				layouter.constrain_instance(result_state[i].cell(), config.results, i)?;
			}
			Ok(())
		}
	}

	#[test]
	fn test_poseidon_x5_5() {
		// Testing 5x5 input.
		let inputs: [Value<Fr>; 5] = [
			"0x0000000000000000000000000000000000000000000000000000000000000000",
			"0x0000000000000000000000000000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000000000000000000000000000002",
			"0x0000000000000000000000000000000000000000000000000000000000000003",
			"0x0000000000000000000000000000000000000000000000000000000000000004",
		]
		.map(|n| Value::known(hex_to_field(n)));

		let outputs: [Fr; 5] = [
			"0x299c867db6c1fdd79dcefa40e4510b9837e60ebb1ce0663dbaa525df65250465",
			"0x1148aaef609aa338b27dafd89bb98862d8bb2b429aceac47d86206154ffe053d",
			"0x24febb87fed7462e23f6665ff9a0111f4044c38ee1672c1ac6b0637d34f24907",
			"0x0eb08f6d809668a981c186beaf6110060707059576406b248e5d9cf6e78b3d3e",
			"0x07748bc6877c9b82c8b98666ee9d0626ec7f5be4205f79ee8528ef1c4a376fc7",
		]
		.map(|n| hex_to_field(n));

		let poseidon_tester = PoseidonTester::new(inputs);

		let k = 7;
		let prover = MockProver::run(k, &poseidon_tester, vec![outputs.to_vec()]).unwrap();
		assert_eq!(prover.verify(), Ok(()));
	}

	#[test]
	fn test_poseidon_x5_5_production() {
		let inputs: [Value<Fr>; 5] = [
			"0x0000000000000000000000000000000000000000000000000000000000000000",
			"0x0000000000000000000000000000000000000000000000000000000000000001",
			"0x0000000000000000000000000000000000000000000000000000000000000002",
			"0x0000000000000000000000000000000000000000000000000000000000000003",
			"0x0000000000000000000000000000000000000000000000000000000000000004",
		]
		.map(|n| Value::known(hex_to_field(n)));

		let outputs: [Fr; 5] = [
			"0x299c867db6c1fdd79dcefa40e4510b9837e60ebb1ce0663dbaa525df65250465",
			"0x1148aaef609aa338b27dafd89bb98862d8bb2b429aceac47d86206154ffe053d",
			"0x24febb87fed7462e23f6665ff9a0111f4044c38ee1672c1ac6b0637d34f24907",
			"0x0eb08f6d809668a981c186beaf6110060707059576406b248e5d9cf6e78b3d3e",
			"0x07748bc6877c9b82c8b98666ee9d0626ec7f5be4205f79ee8528ef1c4a376fc7",
		]
		.map(|n| hex_to_field(n));

		let poseidon_tester = PoseidonTester::new(inputs);

		let k = 7;
		let rng = &mut rand::thread_rng();
		let params = generate_params(k);
		let res =
			prove_and_verify::<Bn256, _, _>(params, poseidon_tester, &[&outputs], rng).unwrap();
		assert!(res);
	}
}
