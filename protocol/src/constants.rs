#[cfg(feature = "prod")]
mod consts {
	/// The number of neighbors the peer can have.
	/// This is also the maximum number of peers that can be connected to the
	/// node.
	pub const MAX_NEIGHBORS: usize = 256;
	/// List of predetermened bootstrap peers.
	pub const BOOTSTRAP_PEERS: [&str; NUM_BOOTSTRAP_PEERS] = [
		"2745hHPZtf8prEv4TUSkLVhZN2tPW6KiYwRfC5yixfU9",
		"C1w5WRwb2G7Whykit4XSng7JMjKvpGudjH2qBUMYFYwu",
		"3kiiXA7hgMUmFh9TcmYXqJWU3WVUnjY4rFzQhNXyqt9H",
		"2XEDBFy8gh32c4F5TVGNRVKMo9NM7eYkXH7iJzf68Vs1",
		"FCKVrmoaXpoyPv9H8EiveWhUf8Greh3mSmkU81QXhRts",
	];
	/// The number of bootstrap peers.
	pub const NUM_BOOTSTRAP_PEERS: usize = 5;
	/// The score of a bootstrap peer.
	pub const BOOTSTRAP_SCORE: f64 = 0.5;
	/// Number of iterations to loop in each epoch.
	pub const NUM_ITERATIONS: u32 = 10;
	/// Epoch duration in seconds
	pub const EPOCH_INTERVAL: u64 = 60 * 60; // One hour
}

#[cfg(not(feature = "prod"))]
#[allow(missing_docs)]
mod consts {
	pub const MAX_NEIGHBORS: usize = 12;
	pub const BOOTSTRAP_PEERS: [&str; NUM_BOOTSTRAP_PEERS] = [
		"2745hHPZtf8prEv4TUSkLVhZN2tPW6KiYwRfC5yixfU9",
		"C1w5WRwb2G7Whykit4XSng7JMjKvpGudjH2qBUMYFYwu",
		"3kiiXA7hgMUmFh9TcmYXqJWU3WVUnjY4rFzQhNXyqt9H",
		"2XEDBFy8gh32c4F5TVGNRVKMo9NM7eYkXH7iJzf68Vs1",
		"FCKVrmoaXpoyPv9H8EiveWhUf8Greh3mSmkU81QXhRts",
	];
	pub const NUM_BOOTSTRAP_PEERS: usize = 5;
	pub const BOOTSTRAP_SCORE: f64 = 0.5;
	pub const NUM_ITERATIONS: u32 = 6;
	pub const EPOCH_INTERVAL: u64 = 100;
	pub const ITER_INTERVAL: u64 = 10;
}

pub use consts::*;
