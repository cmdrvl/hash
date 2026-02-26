# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in hash, please report it privately to the maintainers.

### Reporting Process

1. **Do not** open a public issue for security vulnerabilities
2. Email security concerns to: [security@cmdrvl.com]
3. Include a detailed description of the vulnerability
4. Provide steps to reproduce the issue
5. Include any proof-of-concept code if applicable

### Response Timeline

- We will acknowledge receipt of your report within 48 hours
- We will provide a detailed response within 7 days
- We will work with you to understand and resolve the issue
- We will coordinate disclosure once a fix is available

### Security Features

This tool implements several security measures:

- **Deterministic builds**: Reproducible binary compilation
- **Supply chain attestations**: Build provenance tracking via GitHub Actions
- **SBOM generation**: Complete software bill of materials
- **Checksum verification**: SHA256 checksums for all release artifacts
- **No unsafe code**: Built with `#![forbid(unsafe_code)]`

### Verification

All release artifacts include:
- SHA256 checksums for integrity verification
- SBOM (Software Bill of Materials) in CycloneDX format
- GitHub build provenance attestations
- Signed Git tags for release commits

To verify a release:

```bash
# Download the binary and checksum
curl -L -o hash-v0.1.0-x86_64-unknown-linux-gnu.tar.gz \
  https://github.com/cmdrvl/hash/releases/download/v0.1.0/hash-v0.1.0-x86_64-unknown-linux-gnu.tar.gz

curl -L -o hash-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256 \
  https://github.com/cmdrvl/hash/releases/download/v0.1.0/hash-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify checksum
sha256sum -c hash-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
```