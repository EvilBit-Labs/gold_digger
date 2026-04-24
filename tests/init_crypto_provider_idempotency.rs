//! Idempotency coverage for [`gold_digger::init_crypto_provider`] (todo #081).
//!
//! `src/lib.rs::init_crypto_provider` wraps
//! `rustls::crypto::ring::default_provider().install_default()` in a
//! [`std::sync::Once`]. The intent: any number of callers (tests, library
//! consumers, parallel test threads) can invoke it without panic, and only
//! the first call actually installs the provider.
//!
//! This file pins that contract. The unit suite in `src/lib.rs::tests`
//! cannot easily exercise the multi-call path because the process-global
//! `Once` flips on the very first invocation in any test binary; here we
//! get a fresh process, so the call sequence is fully observable.
//!
//! The follow-up todo `init-crypto-provider-must-surface-or-panic-on-install-error`
//! tracks promoting the swallowed `install_default` error to a typed
//! return value; once that lands, this file should grow a positive
//! "explicit error surfaced" assertion.

use gold_digger::init_crypto_provider;

/// Calling [`init_crypto_provider`] repeatedly must never panic or
/// deadlock. The `Once` guard is the same primitive used by
/// `std::sync::OnceLock`, so a regression here would also indicate a
/// regression in the broader rustls bootstrap path.
#[test]
fn init_crypto_provider_is_idempotent() {
    init_crypto_provider();
    init_crypto_provider();
    init_crypto_provider();
}

/// Multi-threaded idempotency: multiple threads racing into
/// [`init_crypto_provider`] at startup must all complete cleanly
/// without panic and without observable interleaving issues. This is
/// the realistic shape of the call from `cargo nextest`/`cargo test`,
/// where many test binaries spin up thread pools that touch rustls
/// before any single test runs.
#[test]
fn init_crypto_provider_is_thread_safe() {
    let handles: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(init_crypto_provider))
        .collect();
    for handle in handles {
        handle.join().expect("init_crypto_provider thread panicked");
    }
}
