# Security Policy

## Supported Versions

Gold Digger follows semantic versioning. Security updates are provided for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2.0 | :x:                |

## Reporting a Vulnerability

**We take security vulnerabilities seriously.** Please report any security issues you discover.

### Preferred Method: Private Disclosure

1. **Go to the [Security tab](https://github.com/EvilBit-Labs/gold_digger/security)**
2. Click "Report a vulnerability"
3. Fill out the security report form
4. Provide detailed information about the vulnerability

### Alternative Methods

- **Email**: <support@evilbitlabs.io> (for urgent or highly sensitive issues)
- **GitHub Issue**: Use the [Security Report template](https://github.com/EvilBit-Labs/gold_digger/issues/new?template=security_report.yml) (public disclosure - redact sensitive details)

### What to Include

When reporting a vulnerability, please provide:

- **Description**: Clear explanation of the security issue
- **Severity**: Critical, High, Medium, or Low impact
- **Steps to Reproduce**: Detailed reproduction steps
- **Proof of Concept**: Minimal code demonstrating the issue (redact sensitive data)
- **Affected Versions**: Which versions are vulnerable
- **Environment**: OS, architecture, enabled features
- **Impact Assessment**: Potential consequences

### Response Timeline

As a single maintainer project:

- **Critical/High**: 24-48 hours initial response
- **Medium**: 3-5 business days
- **Low**: 1-2 weeks

Security issues are prioritized over feature development.

## Security Features

### Database Security

- **Credential Protection**: Database URLs and credentials are never logged
- **TLS Support**: Secure database connections with native TLS or rustls
- **Connection Validation**: Proper error handling for connection failures
- **No Credential Storage**: Credentials are only read from environment variables

### Input Validation

- **SQL Injection Prevention**: Uses parameterized queries via mysql crate
- **Environment Variable Validation**: Validates required configuration
- **File Path Sanitization**: Validates output file paths

### Output Security

- **File Permissions**: Respects system umask for output files
- **No Sensitive Data in Output**: Database credentials are never included in results
- **Structured Output**: Safe CSV, JSON, and TSV generation

## Security Best Practices

### For Users

1. **Use Environment Variables**: Store database credentials in environment variables, not in scripts
2. **Enable TLS**: Use `mysql://` URLs with SSL parameters for encrypted connections
3. **Limit Permissions**: Use database users with minimal required permissions
4. **Secure Output**: Store output files in secure locations with appropriate permissions
5. **Regular Updates**: Keep Gold Digger updated to the latest version

### For Developers

1. **Security Reviews**: All code changes undergo security review
2. **Dependency Scanning**: Regular vulnerability scanning with `cargo audit`
3. **Secure Defaults**: Security-focused default configurations
4. **Error Handling**: No sensitive information in error messages
5. **Input Validation**: Validate all external inputs

## Security Scanning

### Automated Security Checks

- **CodeQL Analysis**: Automated security scanning via GitHub Actions
- **Dependency Scanning**: Regular vulnerability checks with Dependabot
- **Cargo Audit**: Rust dependency vulnerability scanning
- **SBOM Generation**: Software Bill of Materials for supply chain security

### Manual Security Reviews

- **Code Reviews**: All changes reviewed for security implications
- **Penetration Testing**: Periodic security testing of the application
- **Configuration Audits**: Regular review of security configurations

## Vulnerability Disclosure

### Responsible Disclosure Policy

1. **Private Reporting**: Security issues are reported privately first
2. **Coordinated Disclosure**: Vulnerabilities are disclosed after fixes are available
3. **CVE Assignment**: Critical and high-severity issues receive CVE assignments
4. **Public Disclosure**: Security advisories published with fix details

### Disclosure Timeline

- **Discovery**: Vulnerability is discovered and reported
- **Assessment**: Issue is assessed and severity determined
- **Fix Development**: Security fix is developed and tested
- **Release**: Fixed version is released
- **Disclosure**: Public disclosure with security advisory

## Security Contacts

### Primary Contact

- **Maintainer**: UncleSp1d3r
- **GitHub**: [@unclesp1d3r](https://github.com/unclesp1d3r)
- **Email**: <unclesp1d3r@evilbitlabs.io>

### Security Team

This is a single-maintainer project. All security issues are handled by the primary maintainer.

## Security Acknowledgments

We appreciate security researchers who responsibly disclose vulnerabilities. Contributors to security improvements will be acknowledged in:

- Release notes
- Security advisories
- Project documentation

## Security Resources

- [GitHub Security Policy](https://docs.github.com/en/code-security/getting-started/adding-a-security-policy-to-your-repository)
- [Rust Security WG](https://github.com/rust-secure-code/wg)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CVE Database](https://cve.mitre.org/)

## License

This security policy is part of the Gold Digger project and is licensed under the MIT License.
