pub mod alpha_scalar;
pub mod alpha_vector;
pub mod remote;

#[derive(clap::Subcommand)]
pub enum Command {
    AlphaScalar(alpha_scalar::Command),
    AlphaVector(alpha_vector::Command),
    Remote(remote::Command),
}
