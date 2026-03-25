use clap::{Parser, Subcommand};

/// Rlist - A file management CLI tool
#[derive(Parser, Debug)]
#[command(name = "rlist")]
#[command(about = "A file management tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<RlistSubcommand>,
}

#[derive(Subcommand, Debug)]
pub enum RlistSubcommand {
    /// Manage user passwords
    #[command(subcommand)]
    Passwd(PasswdSubCommand),
    /// Run the server
    Run {
        /// The port to run on
        #[arg(long, default_value_t = 10000, help = "Specify the port to run on")]
        port: u16,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum PasswdSubCommand {
    /// Reset password to a specified new password
    Rst {
        /// The username to reset password for
        #[arg(short = 'u', long, default_value = "admin")]
        user: String,
        #[arg(short = 'n', long)]
        new_password: String,
    },
    /// Generate a random password for a user
    Random {
        /// The username to generate password for
        #[arg(short, long, default_value = "admin")]
        user: String,
    },
}
