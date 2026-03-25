use clap::Subcommand;

#[derive(Subcommand)]
enum AdminSubcommand {
    /// Reset a user's password
    Passwd {
        /// The username to reset password for
        #[arg(help = "Specify the new admin's password")]
        rst: String,

        /// Generate a random password
        #[arg(long, help = "Generate a random password instead of prompting for one")]
        random: bool,
    },
}
