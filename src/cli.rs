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

/// TLS configuration options (mutually exclusive validation modes, plus a
/// second-confirmation flag for `--allow-invalid-certificate`).
#[derive(Args, Debug, Clone)]
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
    ///
    /// **Requires a second confirmation flag:** pass
    /// `--i-understand-this-is-insecure` (or set
    /// `GOLD_DIGGER_ALLOW_INVALID=1`) to actually activate the mode. This
    /// is a guard against the flag being accidentally left in a script;
    /// stderr-only `[DANGER]` warnings are swallowed by `2>/dev/null` in
    /// CI and cron (todo #022, CWE-295 / CWE-296).
    #[arg(long, group = "tls_mode")]
    pub allow_invalid_certificate: bool,

    /// Second-confirmation flag required alongside `--allow-invalid-certificate`.
    ///
    /// Acts as an explicit opt-in that the user understands they are
    /// disabling all TLS certificate validation and accepting full MITM
    /// exposure. Alternatively set the `GOLD_DIGGER_ALLOW_INVALID=1`
    /// environment variable (for ops-managed deployments). Without one of
    /// these, `--allow-invalid-certificate` is treated as a configuration
    /// error (exit 2) rather than silently downgrading security.
    #[arg(long, env = "GOLD_DIGGER_ALLOW_INVALID")]
    pub i_understand_this_is_insecure: bool,
}

impl TlsOptions {
    /// Builds a [`crate::tls::TlsConfig`] from the parsed CLI flags.
    ///
    /// # Why this lives in the CLI layer (#045)
    ///
    /// `TlsConfig` is an infrastructure primitive that should compile
    /// without any `clap`-decorated input type. The adapter from
    /// [`TlsOptions`] → [`crate::tls::TlsConfig`] lives here so the `tls`
    /// module depends only on primitive types (bool + `Option<PathBuf>`),
    /// enabling a future extraction into a sibling crate.
    ///
    /// # Fail-closed second confirmation (#022)
    ///
    /// If `--allow-invalid-certificate` is set WITHOUT either
    /// `--i-understand-this-is-insecure` or
    /// `GOLD_DIGGER_ALLOW_INVALID=1`, this function returns a
    /// [`crate::tls::TlsError::MutuallyExclusiveFlags`] variant (which
    /// maps to exit 2 / config error). The call site is expected to
    /// surface this error verbatim — stderr `[DANGER]` warnings are
    /// swallowed by `2>/dev/null` in CI / cron, so structural
    /// enforcement is the only reliable signal.
    pub fn to_tls_config(&self) -> Result<crate::tls::TlsConfig, crate::tls::TlsError> {
        if self.allow_invalid_certificate && !self.i_understand_this_is_insecure {
            return Err(crate::tls::TlsError::MutuallyExclusiveFlags {
                flags:
                    "--allow-invalid-certificate requires the companion flag --i-understand-this-is-insecure (or env GOLD_DIGGER_ALLOW_INVALID=1).                      Refusing to disable certificate validation without explicit confirmation. See SECURITY.md (todo #022)."
                        .to_string(),
            });
        }

        crate::tls::TlsConfig::from_cli_args(
            self.tls_ca_file.as_ref(),
            self.insecure_skip_hostname_verify,
            self.allow_invalid_certificate,
        )
    }
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
