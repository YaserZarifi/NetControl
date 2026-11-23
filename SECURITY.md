# Security policy

NetControl is under active development. No stable release has been published yet.
Security fixes will target the latest release and the default branch.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting feature on the repository's Security
tab when available. If private reporting is unavailable, open an issue requesting
a private contact method without including exploit details or private information.
Do not publish credentials, captured traffic, or personal application paths.

## Security boundaries

The desktop interface must not receive arbitrary elevated shell execution access.
Administrator operations must be limited to the requested monitoring or firewall
action. Firewall changes must identify rules owned by NetControl and preserve all
other rules. Browser preview must never perform Windows administration.

Release downloads belong on this repository's GitHub Releases page. Verify any
published checksum against the exact installer before running it.
