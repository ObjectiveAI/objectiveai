pub mod generate_secret_signature_pair;
pub mod kill;
pub mod send;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    GenerateSecretSignaturePair(generate_secret_signature_pair::Command),
    Kill(kill::Command),
    Send(send::Command),
    Spawn(spawn::Command),
}
