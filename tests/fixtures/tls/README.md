# TLS Certificate Fixtures

This directory contains TLS certificate fixtures for integration testing.

## Certificate Generation

Certificates are generated ephemeral per test run to ensure security and avoid certificate reuse
across test executions.

## Files

- `ca_cert.pem` - Certificate Authority certificate (generated per test run)
- `server_cert.pem` - Server certificate (generated per test run)
- `server_key.pem` - Server private key (generated per test run)
- `client_cert.pem` - Client certificate (generated per test run)
- `client_key.pem` - Client private key (generated per test run)

## Security Notes

- All certificates are self-signed and for testing only
- Certificates are generated with ephemeral keys per test execution
- No production certificates should be stored in this directory
