# Contributing to NetControl

Report reproducible bugs through GitHub Issues. Include your Windows version,
NetControl version, steps to reproduce, expected behavior, and actual behavior.
Remove private application paths and network information from logs before sharing.

Discuss substantial features in an issue before implementing them. Keep changes
focused and include checks for accounting, persistence, and security behavior.

## Development

Install Node.js 22 or later. Run `npm ci`, then `npm run dev` for a browser preview.
Windows functionality requires the desktop build and cannot be tested in a browser.
Native development requires stable Rust and the Windows prerequisites for Tauri 2.
Contributors may instead use the repository's Windows build workflow.

Before submitting a pull request, run `npm run check` and `npm run build`. Native
changes must also pass Rust formatting, Clippy, and tests as described in
[testing](docs/testing.md). Do not substitute generated or estimated traffic for
actual measurements. Document measurement coverage and privilege requirements.

Use descriptive commit messages. Contributions are licensed under the MIT license.
