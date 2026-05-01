use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "birthday-reminders")]
#[command(about = "Birthday reminder application with web UI and notifications")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    pub config: String,

    /// Enable debug output to stderr
    #[arg(long, global = true)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the web server
    Serve {
        /// Override listen address
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Create a new user (direct DB, no server needed)
    CreateUser {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
        /// Create as admin
        #[arg(long)]
        admin: bool,
    },

    /// Add a birthday (requires --token)
    Add {
        /// Person's name
        name: String,
        /// Birth date (YYYY-MM-DD)
        date: String,
        /// Optional phone number
        #[arg(long)]
        phone_number: Option<String>,
        /// Optional address
        #[arg(long)]
        address: Option<String>,
        /// Optional postal code
        #[arg(long)]
        postal_code: Option<String>,
        /// Optional city
        #[arg(long)]
        city: Option<String>,
        /// Optional country
        #[arg(long)]
        country: Option<String>,
        /// Optional notes
        #[arg(long)]
        notes: Option<String>,
        /// API token
        #[arg(long, env = "BIRTHDAY_API_TOKEN")]
        token: String,
    },

    /// List all birthdays (requires --token)
    List {
        /// API token
        #[arg(long, env = "BIRTHDAY_API_TOKEN")]
        token: String,
    },

    /// Show upcoming birthdays (requires --token)
    Upcoming {
        /// Days ahead to show
        #[arg(long, default_value = "30")]
        days: u32,
        /// API token
        #[arg(long, env = "BIRTHDAY_API_TOKEN")]
        token: String,
    },

    /// Remove a birthday by ID (requires --token)
    Remove {
        /// Birthday ID (UUID)
        id: String,
        /// API token
        #[arg(long, env = "BIRTHDAY_API_TOKEN")]
        token: String,
    },

    /// Manually trigger reminder check for all users
    CheckReminders,
}
