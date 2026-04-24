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
    /// Path-safety guards (todo #023): the path is canonicalized via
    /// `std::fs::canonicalize` before the file is read, which rejects
    /// symlinks that cross filesystem boundaries and resolves traversal
    /// components (`..`). The extension must be `.sql`, `.txt`, or
    /// missing; recognised executable extensions (`.exe`, `.dll`, `.so`,
    /// `.dylib`, `.bin`) are refused with a configuration error to stop
    /// accidental reads of binaries as SQL. Files larger than 10 MiB are
    /// refused to cap DoS risk. Do not run gold_digger as `root` with
    /// `--query-file` pointing at externally-supplied paths, and prefer a
    /// dedicated directory owned by the gold_digger user for query files.
    /// See SECURITY.md.
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

    /// Overwrite the output file if it already exists.
    ///
    /// By default, gold_digger refuses to clobber an existing output file
    /// (path-safety: todo #024). It opens the output with
    /// `O_CREAT | O_EXCL` semantics on Unix so a pre-existing file — or a
    /// symlink at the target path — is a hard error. Pass `--force` to
    /// request explicit overwrite; symlinks at the target are still
    /// refused on Unix via `O_NOFOLLOW`. This guards against an attacker
    /// pre-placing a symlink at a predictable output path (e.g.
    /// `/tmp/results.json`) and redirecting the write to an
    /// attacker-chosen target. See SECURITY.md.
    #[arg(long)]
    pub force: bool,

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
    /// Infers an output format from the file extension.
    ///
    /// Matching is case-insensitive (`.CSV`, `.Json`, `.TSV` all recognised)
    /// so Windows path-preservation does not silently flip the output format.
    /// Returns `None` for missing or unrecognised extensions so the caller
    /// can surface a clear "specify --format" error instead of silently
    /// defaulting to TSV (see todo #019; fail-fast per coding-style rule).
    pub fn from_extension(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension().and_then(|s| s.to_str())?;
        match ext.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "tsv" | "tab" | "txt" => Some(Self::Tsv),
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn from_extension_csv_lowercase() {
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("out.csv")),
            Some(OutputFormat::Csv)
        ));
    }

    #[test]
    fn from_extension_json_lowercase() {
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("out.json")),
            Some(OutputFormat::Json)
        ));
    }

    #[test]
    fn from_extension_tsv_lowercase() {
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("out.tsv")),
            Some(OutputFormat::Tsv)
        ));
    }

    #[test]
    fn from_extension_uppercase_is_case_insensitive() {
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("OUT.CSV")),
            Some(OutputFormat::Csv)
        ));
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("OUT.JSON")),
            Some(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::from_extension(&PathBuf::from("OUT.Tsv")),
            Some(OutputFormat::Tsv)
        ));
    }

    #[test]
    fn from_extension_unknown_returns_none() {
        assert!(OutputFormat::from_extension(&PathBuf::from("out.xml")).is_none());
        assert!(OutputFormat::from_extension(&PathBuf::from("out.yaml")).is_none());
        assert!(OutputFormat::from_extension(&PathBuf::from("out.data")).is_none());
    }

    #[test]
    fn from_extension_missing_returns_none() {
        assert!(OutputFormat::from_extension(&PathBuf::from("out")).is_none());
        assert!(OutputFormat::from_extension(&PathBuf::from("")).is_none());
    }
}
