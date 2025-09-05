# Release Artifact Verification

This document provides instructions for verifying the integrity and authenticity of Gold Digger release artifacts.

## Overview

Each Gold Digger release includes the following security artifacts:

- **Binaries**: Cross-platform executables for Linux, macOS, and Windows
- **Checksums**: SHA256 checksums for all binaries (`SHA256SUMS` and individual `.sha256` files)
- **SBOMs**: Software Bill of Materials in CycloneDX format (`.sbom.cdx.json` files)
- **Signatures**: Cosign keyless signatures (`.sig` and `.crt` files)

## Checksum Verification

### Using the Consolidated Checksums File

1. Download the `SHA256SUMS` file from the release
2. Download the binary you want to verify
3. Verify the checksum:

```bash
# Linux/macOS
sha256sum -c SHA256SUMS

# Or verify a specific file
sha256sum gold_digger-x86_64-unknown-linux-gnu.tar.gz
# Compare with the value in SHA256SUMS
```

### Using Individual Checksum Files

Each binary has a corresponding `.sha256` file:

```bash
# Download both the binary and its checksum file
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify the checksum
sha256sum -c gold_digger-x86_64-unknown-linux-gnu.tar.gz.sha256
```

## Signature Verification

Gold Digger releases are signed using [Cosign](https://github.com/sigstore/cosign) with keyless OIDC authentication.

### Install Cosign

```bash
# Linux/macOS
curl -O -L "https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64"
sudo mv cosign-linux-amd64 /usr/local/bin/cosign
sudo chmod +x /usr/local/bin/cosign

# Or use package managers
# Homebrew (macOS/Linux)
brew install cosign

# APT (Ubuntu/Debian)
sudo apt-get update
sudo apt install cosign
```

### Verify Signatures

Each binary has corresponding `.sig` (signature) and `.crt` (certificate) files:

```bash
# Download the binary and its signature files
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz.sig
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt

# Verify the signature
cosign verify-blob \
  --certificate gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt \
  --signature gold_digger-x86_64-unknown-linux-gnu.tar.gz.sig \
  --certificate-identity-regexp "^https://github\.com/EvilBit-Labs/gold_digger/\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$" \
  --certificate-oidc-issuer-regexp "^https://token\.actions\.githubusercontent\.com$" \
  gold_digger-x86_64-unknown-linux-gnu.tar.gz
```

### Understanding the Certificate

The certificate contains information about the signing identity:

```bash
# Examine the certificate
openssl x509 -in gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt -text -noout
```

Look for:

- **Subject**: Should contain GitHub Actions workflow information
- **Issuer**: Should be from Sigstore/Fulcio
- **SAN (Subject Alternative Name)**: Should contain the GitHub repository URL

### Extracting Certificate Identity and Issuer

To extract the exact certificate identity and issuer values for verification:

```bash
# Extract the certificate identity (SAN URI)
openssl x509 -in gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt -text -noout | grep -A1 "X509v3 Subject Alternative Name" | grep URI

# Extract the OIDC issuer
openssl x509 -in gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt -text -noout | grep -A10 "X509v3 extensions" | grep -A5 "1.3.6.1.4.1.57264.1.1" | grep "https://token.actions.githubusercontent.com"
```

The certificate identity should match the pattern:
`https://github.com/EvilBit-Labs/gold_digger/.github/workflows/release.yml@refs/tags/v1.0.0`

The OIDC issuer should be:
`https://token.actions.githubusercontent.com`

**Security Note**: The verification commands in this documentation use exact regex patterns anchored to these specific values to prevent signature forgery attacks. Never use wildcard patterns like `.*` in production verification.

## SBOM Inspection

Software Bill of Materials (SBOM) files provide detailed information about dependencies and components.

### Install SBOM Tools

```bash
# Install syft for SBOM generation and inspection
curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin

# Install grype for vulnerability scanning
curl -sSfL https://raw.githubusercontent.com/anchore/grype/main/install.sh | sh -s -- -b /usr/local/bin
```

### Inspect SBOM Contents

```bash
# Download the SBOM file
wget https://github.com/EvilBit-Labs/gold_digger/releases/download/v1.0.0/gold_digger-x86_64-unknown-linux-gnu.tar.gz.sbom.cdx.json

# View SBOM in human-readable format
syft packages file:gold_digger-x86_64-unknown-linux-gnu.tar.gz.sbom.cdx.json -o table

# View detailed JSON structure
jq . gold_digger-x86_64-unknown-linux-gnu.tar.gz.sbom.cdx.json | less
```

### Vulnerability Assessment

Use the SBOM to check for known vulnerabilities:

```bash
# Scan the SBOM for vulnerabilities
grype sbom:gold_digger-x86_64-unknown-linux-gnu.tar.gz.sbom.cdx.json

# Generate a vulnerability report
grype sbom:gold_digger-x86_64-unknown-linux-gnu.tar.gz.sbom.cdx.json -o json > vulnerability-report.json
```

## Complete Verification Script

Here's a complete script that verifies all aspects of a release artifact:

```bash
#!/bin/bash
set -euo pipefail

RELEASE_TAG="v1.0.0"
ARTIFACT_NAME="gold_digger-x86_64-unknown-linux-gnu.tar.gz"
BASE_URL="https://github.com/EvilBit-Labs/gold_digger/releases/download/${RELEASE_TAG}"

echo "🔍 Verifying Gold Digger release artifact: ${ARTIFACT_NAME}"

# Download all required files
echo "📥 Downloading files..."
wget -q "${BASE_URL}/${ARTIFACT_NAME}"
wget -q "${BASE_URL}/${ARTIFACT_NAME}.sha256"
wget -q "${BASE_URL}/${ARTIFACT_NAME}.sig"
wget -q "${BASE_URL}/${ARTIFACT_NAME}.crt"
wget -q "${BASE_URL}/${ARTIFACT_NAME}.sbom.cdx.json"

# Verify checksum
echo "🔐 Verifying checksum..."
if sha256sum -c "${ARTIFACT_NAME}.sha256"; then
    echo "✅ Checksum verification passed"
else
    echo "❌ Checksum verification failed"
    exit 1
fi

# Verify signature
echo "🔏 Verifying signature..."
if cosign verify-blob \
    --certificate "${ARTIFACT_NAME}.crt" \
    --signature "${ARTIFACT_NAME}.sig" \
    --certificate-identity "https://github.com/EvilBit-Labs/gold_digger/.github/workflows/release.yml@refs/tags/${RELEASE_TAG}" \
    --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
    "${ARTIFACT_NAME}"; then
    echo "✅ Signature verification passed"
else
    echo "❌ Signature verification failed"
    exit 1
fi

# Validate SBOM
echo "📋 Validating SBOM..."
if jq empty "${ARTIFACT_NAME}.sbom.cdx.json" 2>/dev/null; then
    echo "✅ SBOM is valid JSON"

    # Show SBOM summary
    echo "📊 SBOM Summary:"
    syft packages "sbom:${ARTIFACT_NAME}.sbom.cdx.json" -o table | head -20
else
    echo "❌ SBOM validation failed"
    exit 1
fi

echo "🎉 All verifications passed! The artifact is authentic and secure."
```

## Airgap Installation Guide

For environments without internet access:

### 1. Download Required Files

On a connected machine, download:

- The binary archive
- The `.sha256` checksum file
- The `.sig` and `.crt` signature files
- The `.sbom.cdx.json` SBOM file

### 2. Transfer to Airgap Environment

Transfer all files to the airgap environment using approved methods (USB, secure file transfer, etc.).

### 3. Verify in Airgap Environment

```bash
# Verify checksum (no network required)
# For Linux (GNU coreutils):
sha256sum -c gold_digger-x86_64-unknown-linux-gnu.tar.gz.sha256

# For macOS (native):
shasum -a 256 -c gold_digger-x86_64-unknown-linux-gnu.tar.gz.sha256

# Note: If sha256sum is not available on macOS, install GNU coreutils:
# brew install coreutils

# Extract and install
tar -xzf gold_digger-x86_64-unknown-linux-gnu.tar.gz
sudo mv gold_digger /usr/local/bin/
sudo chmod +x /usr/local/bin/gold_digger

# Verify installation
gold_digger --version
```

### 4. Optional: Offline Signature Verification

If Cosign is available in the airgap environment:

```bash
# Verify signature (requires Cosign but no network)
cosign verify-blob \
  --certificate gold_digger-x86_64-unknown-linux-gnu.tar.gz.crt \
  --signature gold_digger-x86_64-unknown-linux-gnu.tar.gz.sig \
  --certificate-identity-regexp "^https://github\.com/EvilBit-Labs/gold_digger/\.github/workflows/release\.yml@refs/tags/v[0-9]+\.[0-9]+\.[0-9]+$" \
  --certificate-oidc-issuer-regexp "^https://token\.actions\.githubusercontent\.com$" \
  gold_digger-x86_64-unknown-linux-gnu.tar.gz
```

## Security Considerations

### Trust Model

- **Signatures**: Trust is rooted in GitHub's OIDC identity and Sigstore's transparency log
- **Checksums**: Protect against corruption and tampering
- **SBOMs**: Enable vulnerability assessment and supply chain analysis

### Verification Best Practices

1. **Always verify checksums** before using any binary
2. **Verify signatures** when possible to ensure authenticity
3. **Review SBOMs** for security-sensitive deployments
4. **Use the latest release** unless you have specific version requirements
5. **Report security issues** through GitHub's security advisory process

### Automated Verification

For CI/CD pipelines, consider automating verification:

```yaml
  - name: Verify Gold Digger Release
    run: |
      # Download and verify as shown above
      # Fail the pipeline if verification fails
```

## Troubleshooting

### Common Issues

**Checksum mismatch**: Re-download the file, check for network issues
**Signature verification fails**: Ensure you have the correct certificate and signature files
**SBOM parsing errors**: Verify the SBOM file wasn't corrupted during download

### Getting Help

- **Security issues**: Use GitHub's security advisory process
- **General questions**: Open an issue on the GitHub repository
- **Documentation**: Check the main documentation at `/docs/`
