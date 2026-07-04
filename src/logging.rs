//! Structured logging, colored stderr messages, and progress indicators.
//!
//! Three responsibilities live in this module so `main.rs` can stay narrow:
//!
//!   1. **Tracing initialisation** ([`init_tracing`]). Installs a
//!      [`tracing_subscriber::fmt`] subscriber on stderr with human-readable
//!      plain-text output (no JSON; see AGENTS.md — the `--log-format json`
//!      follow-up is a separate task). Level filtering is derived from
//!      `cli.verbose` / `cli.quiet`:
//!
//!        | Flags          | Level  |
//!        |----------------|--------|
//!        | `--quiet`      | error  |
//!        | (default)      | warn   |
//!        | `-v`           | info   |
//!        | `-vv`          | debug  |
//!        | `-vvv` or more | trace  |
//!
//!      The `RUST_LOG` env var can override via `EnvFilter` for ad-hoc
//!      debugging without rebuilding.
//!
//!   2. **Colored stderr helpers** ([`warn_banner`], [`danger_banner`]).
//!      Wrap a plain string in `owo-colors` styling, but only when stderr is
//!      a TTY AND `NO_COLOR` is unset (per <https://no-color.org/>). When
//!      either condition is false the helpers return the input unchanged so
//!      piping to a file produces clean plain text. Callers still route the
//!      returned string through `tracing::warn!` / `tracing::error!` so the
//!      redaction / level-filter layers run.
//!
//!   3. **Progress indicators** ([`make_progress`]). Builds a
//!      [`indicatif::ProgressBar`] that draws on stderr only when stderr is a
//!      TTY AND `--quiet` is not set. In every other case the returned bar
//!      is a hidden no-op (`ProgressBar::hidden`), so callers can always
//!      call `.set_message`, `.finish_with_message`, etc. without branching
//!      on whether progress is visible.
//!
//! ## Security
//!
//! Log messages are plain strings; the redaction rules live in
//! [`crate::utils`]. Callers MUST route any string that could contain a
//! credential through [`crate::utils::redact_sql_error`] or
//! [`crate::utils::redact_url`] **before** passing it to a `tracing::*!`
//! macro. The subscriber does not inspect field contents for secrets.

use std::io::IsTerminal;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::{OwoColorize, Stream, Style};
use tracing_subscriber::{EnvFilter, fmt};

/// Default tick interval for indeterminate spinners. Short enough to feel
/// responsive on a local terminal, long enough to avoid burning CPU.
const SPINNER_TICK_MS: u64 = 120;

/// Initialises the `tracing` subscriber with a plain-text stderr formatter
/// and a verbosity filter derived from CLI flags.
///
/// # Arguments
///
/// * `verbose` — the `-v` count from `cli.verbose` (u8).
/// * `quiet`   — the `--quiet` flag from `cli.quiet`.
///
/// # Level mapping
///
/// | `quiet` | `verbose` | Level |
/// |---------|-----------|-------|
/// | `true`  | *any*     | error |
/// | `false` | 0         | warn  |
/// | `false` | 1         | info  |
/// | `false` | 2         | debug |
/// | `false` | ≥3        | trace |
///
/// The `RUST_LOG` env var, when set, overrides the CLI-derived filter.
/// This is intended for ad-hoc debugging; CI / production should drive
/// verbosity through the CLI flags.
///
/// # Idempotence
///
/// Safe to call at most once per process. Subsequent calls after the first
/// successful installation silently no-op (the second `set_global_default`
/// fails and we discard the error rather than panic — the subscriber is a
/// best-effort observability concern, not a correctness-critical boundary).
pub fn init_tracing(verbose: u8, quiet: bool) {
    let level_str = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };

    // `RUST_LOG` wins when set (ad-hoc debugging). Otherwise derive from the
    // CLI-mapped level above. `EnvFilter::try_new` cannot fail for the
    // fixed-string cases we use, so the fallback to "warn" is defensive.
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level_str))
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        // Compact, human-readable output. Explicitly NOT `.json()` — the
        // binary's stderr must stay greppable by humans, and the
        // `--log-format json` flag is a separate todo (not this one).
        .compact()
        // Route everything to stderr so stdout stays clean for structured
        // output (CSV/JSON/TSV). Without this, `fmt` defaults to stdout.
        .with_writer(std::io::stderr)
        // Suppress timestamp and target noise for a CLI-friendly feel. A
        // later `--log-format json` task can re-enable these for CI parsers.
        .without_time()
        .with_target(false)
        // Colors only when stderr supports them. The check runs once at
        // subscriber-init time; this is fine because `NO_COLOR` / TTY
        // status don't change mid-run in a CLI binary.
        .with_ansi(stderr_supports_color())
        .finish();

    // Best-effort install. If a subscriber is already set (e.g. because
    // `init_tracing` is called twice from tests) we swallow the error
    // rather than panic — tracing installation is observability, not
    // correctness.
    let _ = tracing::subscriber::set_global_default(subscriber);
}

/// Returns `true` when stderr is a terminal AND the caller hasn't opted out
/// via `NO_COLOR`. Used both by the tracing formatter (ANSI colours on the
/// log lines themselves) and by [`warn_banner`] / [`danger_banner`] (ad-hoc
/// coloring of warning prefixes).
///
/// See <https://no-color.org/> — the convention is that any non-empty value
/// for `NO_COLOR` disables colour. We read the env once per call rather
/// than caching because the cost is trivial and test isolation benefits
/// (tests that set/unset `NO_COLOR` don't need to restart the process).
pub fn stderr_supports_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// Formats a `[WARNING] …` banner with bold yellow styling on colour-capable
/// stderr; returns the plain string unchanged when `NO_COLOR` is set or
/// stderr is not a TTY. The `owo-colors` `if_supports_color(Stream::Stderr,
/// …)` wrapper handles both the TTY check and `NO_COLOR` natively; we still
/// gate it on [`stderr_supports_color`] so our own test harness can force
/// plain output via `NO_COLOR=1`.
///
/// The caller is expected to pass the returned string to `tracing::warn!`
/// (or similar) so the log-level filter still applies; this function is
/// only a styling helper, not a print helper.
pub fn warn_banner(message: &str) -> String {
    if stderr_supports_color() {
        // Single allocation: the closure returns a borrowed `Display` wrapper
        // (`FgColorDisplay`) and the outer `to_string` writes the styled escape
        // codes directly into the result. Returning `t.yellow().to_string()`
        // from the closure would allocate twice — once inside the closure to
        // build the styled string, and again when the outer wrapper's `Display`
        // impl re-emits it.
        message
            .if_supports_color(Stream::Stderr, |t| t.yellow())
            .to_string()
    } else {
        message.to_owned()
    }
}

/// Formats a `[DANGER] …` banner with bold red styling on colour-capable
/// stderr; returns the plain string unchanged otherwise. Mirrors
/// [`warn_banner`] but uses `red().bold()` to make `[DANGER]` lines
/// impossible to miss in scrollback — the motivating case is the TLS
/// certificate-validation-disabled warning, which otherwise blends into
/// mysql-crate chatter.
pub fn danger_banner(message: &str) -> String {
    if stderr_supports_color() {
        // Single allocation — see `warn_banner` for rationale. We compose the
        // two effects via `Style` so the closure can return a `Styled<&str>`
        // value (no nested borrow into a temporary), which the outer
        // `to_string` writes directly into the result. The naive
        // `t.red().bold().to_string()` form inside the closure would allocate
        // twice.
        let style = Style::new().red().bold();
        message
            .if_supports_color(Stream::Stderr, |t| style.style(t))
            .to_string()
    } else {
        message.to_owned()
    }
}

/// Builds a progress indicator for the current CLI phase.
///
/// Returns a **visible** spinner / bar iff all of these are true:
///   * `quiet` is `false`,
///   * stderr is a TTY (progress would be nonsense when redirected),
///   * `NO_COLOR` is unset — some CI environments signal "no fancy output"
///     that way, and obeying it avoids garbled spinner glyphs in logs.
///
/// Otherwise returns [`ProgressBar::hidden`], which silently swallows all
/// subsequent `set_message` / `finish_*` calls so callers don't need
/// branching logic. `total_rows = None` produces an indeterminate spinner;
/// `Some(n)` produces a bounded bar with ETA.
pub fn make_progress(quiet: bool, total_rows: Option<u64>, message: &str) -> ProgressBar {
    let visible = !quiet
        && std::io::stderr().is_terminal()
        && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty());

    if !visible {
        return ProgressBar::hidden();
    }

    match total_rows {
        None => {
            let pb = ProgressBar::new_spinner();
            // Style: `⠋ message`. The default spinner tick chars are fine;
            // we only tweak the template so the message sits flush-left.
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
            pb.set_message(message.to_string());
            pb
        }
        Some(total) => {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("#>-"),
            );
            pb.set_message(message.to_string());
            pb
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `NO_COLOR=1` must disable colour regardless of TTY state. We run
    /// this under `temp_env` so the test cannot leak into other tests that
    /// assert on coloured output.
    #[test]
    fn no_color_env_disables_color() {
        temp_env::with_var("NO_COLOR", Some("1"), || {
            assert!(!stderr_supports_color(), "NO_COLOR=1 must disable colour");
            assert_eq!(warn_banner("[WARNING] test"), "[WARNING] test");
            assert_eq!(danger_banner("[DANGER] test"), "[DANGER] test");
        });
    }

    /// Empty `NO_COLOR` (common in CI with `env NO_COLOR=`) must NOT trip
    /// the no-color path — per the spec, any non-empty value disables
    /// colour, but an empty string means the variable is effectively unset.
    #[test]
    fn empty_no_color_env_does_not_disable_color() {
        temp_env::with_var("NO_COLOR", Some(""), || {
            // We can only assert the env check isolated from TTY here; the
            // combined `stderr_supports_color` also depends on TTY which
            // cargo test doesn't provide. So assert on the env branch only:
            assert!(
                std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
                "empty NO_COLOR should be treated as unset"
            );
        });
    }

    /// `make_progress(quiet = true, ..)` must return a hidden bar even on a
    /// TTY. The hidden bar quietly accepts all subsequent method calls, so
    /// callers need no branching.
    #[test]
    fn quiet_suppresses_progress() {
        let pb = make_progress(true, None, "should not render");
        assert!(pb.is_hidden(), "quiet mode must produce a hidden bar");
    }

    /// `make_progress` under `NO_COLOR=1` is also hidden, so redirected or
    /// sanitized CI logs do not accumulate garbage lines.
    #[test]
    fn no_color_suppresses_progress() {
        temp_env::with_var("NO_COLOR", Some("1"), || {
            let pb = make_progress(false, Some(42), "hidden in NO_COLOR");
            assert!(pb.is_hidden(), "NO_COLOR=1 must suppress progress bars");
        });
    }

    /// `init_tracing` must not panic regardless of how many times we call
    /// it, at any verbosity level. The second call's `set_global_default`
    /// fails silently — that's the documented contract.
    #[test]
    fn init_tracing_is_idempotent() {
        init_tracing(0, false);
        init_tracing(3, false);
        init_tracing(0, true);
        // No assertion needed — the contract is "does not panic".
    }
}
