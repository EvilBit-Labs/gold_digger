//! Clap-derive definitions for the `gold_digger` CLI.
//!
//! Resolution precedence is CLI flags > environment variables > error.
//! Fields that accept both (`db_url`, `output`) use clap's `env` attribute
//! so the fallback shows up in `--help`. Subcommands live under [`Commands`]
//! and output formats under [`OutputFormat`]; `completion` generates shell
//! completion scripts via `clap_complete`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// MySQL/MariaDB query tool with structured output
#[derive(Parser)]
#[command(name = "gold_digger")]
#[command(about = "A MySQL/MariaDB query tool that exports results to structured data files")]
#[command(version)]
pub struct Cli {
    /// Database connection URL (mysql://user:pass@host:port/db)
    #[arg(long, env = "DATABASE_URL", value_name = "URL")]
    pub db_url: Option<String>,

    /// SQL query string to execute
    #[arg(short = 'q', long, conflicts_with = "query_file", value_name = "SQL")]
    pub query: Option<String>,

    /// File containing SQL query to execute.
    ///
    /// Reads any file the gold_digger process can read; there is no
    /// allowlist or sandbox today (tracked under repo todo #023). Avoid
    /// passing untrusted paths and do not run gold_digger as `root` with
    /// this flag against externally-supplied paths. See SECURITY.md.
    #[arg(long, conflicts_with = "query", value_name = "FILE")]
    pub query_file: Option<PathBuf>,

    /// Output file path (format inferred from extension: .csv, .json, .tsv)
    #[arg(short, long, env = "OUTPUT_FILE", value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Output format override (ignores file extension)
    #[arg(short = 'f', long, value_enum, value_name = "FORMAT")]
    pub format: Option<OutputFormat>,

    /// Enable verbose logging (-v for info, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Pretty-print output (only applies to JSON format)
    #[arg(short = 'p', long)]
    pub pretty: bool,

    /// Exit successfully on empty result sets
    #[arg(long)]
    pub allow_empty: bool,

    /// Print current configuration as JSON and exit.
    ///
    /// Output uses a best-effort credential redactor (URL passwords,
    /// `password=`, `token=`, `api_key=`, `identified by`). It does NOT
    /// catch arbitrary base64/hex/JWT secrets or non-English secret
    /// labels. Review the JSON before sharing in bug reports or chat.
    #[arg(long)]
    pub dump_config: bool,

    /// TLS configuration options
    #[command(flatten)]
    pub tls_options: TlsOptions,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate shell completion scripts
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

/// TLS configuration options (mutually exclusive)
#[derive(Args, Debug, Clone)]
#[group(id = "tls_mode", multiple = false)]
pub struct TlsOptions {
    /// Path to CA certificate file for trust anchor pinning
    #[arg(long, group = "tls_mode")]
    pub tls_ca_file: Option<PathBuf>,

    /// Skip hostname verification (keeps chain and time validation)
    #[arg(long, group = "tls_mode")]
    pub insecure_skip_hostname_verify: bool,

    /// Disable certificate validation entirely (DANGEROUS — full MITM exposure).
    ///
    /// Disables BOTH the certificate chain and hostname checks. Any
    /// network attacker who can intercept the TCP connection can present
    /// an attacker-controlled certificate, complete the TLS handshake,
    /// and read or modify the database protocol — including plaintext
    /// credentials. Never use against a production database. Prefer
    /// `--tls-ca-file <path>` for self-signed CAs (full validation
    /// against an explicit anchor) or `--insecure-skip-hostname-verify`
    /// for hostname-only mismatches. See SECURITY.md.
    #[arg(long, group = "tls_mode")]
    pub allow_invalid_certificate: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Csv,
    Json,
    Tsv,
}

impl OutputFormat {
    pub fn from_extension(path: &std::path::Path) -> Self {
        match path.extension().and_then(|s| s.to_str()) {
            Some("csv") => Self::Csv,
            Some("json") => Self::Json,
            _ => Self::Tsv, // Default fallback
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Tsv => "tsv",
        }
    }
}
