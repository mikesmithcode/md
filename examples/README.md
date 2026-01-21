To run an example:

## Running an example

`rust
cargo run --example <example_name>
`

## Setup

The setup.rs is a good place to start to configure starting coordinates and velocities. You can however use a previous timesteps file to get started. Code will look for the latest timestep file in the output/snapshots directory.

## Adding a new example

To add an example you must add something like this to the Cargo.toml file in md_sim.

[[example]]
name = "example_code"
path = "../examples/example_code.rs"

