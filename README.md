# fastmail-cli

[![CI](https://github.com/lutra/fastmail-CLI/actions/workflows/ci.yml/badge.svg)](https://github.com/lutra/fastmail-CLI/actions/workflows/ci.yml)
[![CodeQL](https://github.com/lutra/fastmail-CLI/actions/workflows/codeql.yml/badge.svg)](https://github.com/lutra/fastmail-CLI/actions/workflows/codeql.yml)

A command-line interface for Fastmail, designed for automation and AI agents.

## Features

- **Email operations**: list, read, delete
- **Masked Email**: full support for Fastmail's masked email feature
- **Contacts & Calendar**: CalDAV and CardDAV support
- **Files**: WebDAV file management
- **Agent-first**: JSON output, exit codes, and safety mechanisms
- **Safety-first**: `--force`, `--confirm`, whitelist, and dry-run modes
- **Interactive setup**: `fastmail setup` for easy credential configuration

## Installation

### From binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/lutra/fastmail-CLI/releases).

### From source

```bash
cargo install --git https://github.com/lutra/fastmail-CLI fastmail-cli
```

### From crates.io (coming soon)

```bash
cargo install fastmail-cli
```

## Quick Start

### 1. Setup

Run the interactive setup command:

```bash
fastmail setup
```

This will prompt you for your Fastmail API token and save it to `~/.config/fastmail-cli/config.toml`.

Get an API token at: https://app.fastmail.com/settings/security/integrations

### 2. Verify installation

```bash
fastmail mail list --limit 5
```

## Usage

### Email operations

```bash
# List emails
fastmail mail list --limit 10

# Read an email
fastmail mail read <email-id>

# Delete emails (with safety checks)
fastmail mail delete <id> --force
```

### Masked emails

```bash
# List all
fastmail masked list

# Create new
fastmail masked create https://example.com --description "Shopping site"

# Enable/disable
fastmail masked enable <id>
fastmail masked disable <id>

# Delete
fastmail masked delete <id> --force
```

### Contacts (CardDAV)

```bash
# List all contacts
fastmail contacts list

# Create a contact
fastmail contacts create "John Doe" --email "john@example.com"
```

### Calendar (CalDAV)

```bash
# List calendars
fastmail calendar list

# List events
fastmail calendar list-events <calendar-id>
```

### Files (WebDAV)

```bash
# List files
fastmail files list

# Upload a file
fastmail files upload ./document.txt /Documents/

# Download a file
fastmail files download /Documents/report.txt ./report.txt
```

### Whitelist (for send safety)

```bash
fastmail config allow-recipient add team@company.com
fastmail config allow-recipient list
fastmail config allow-remove remove team@company.com
```

## Configuration

Credentials are stored in `~/.config/fastmail-cli/config.toml`:

```toml
[auth]
token = "your-api-token"

[dav]
caldav_url = "https://caldav.fastmail.com/"
carddav_url = "https://carddav.fastmail.com/"
webdav_url = "https://www.fastmail.com/"
```

You can also set the `FASTMAIL_TOKEN` environment variable as an alternative.

For CalDAV/CardDAV operations, you need an app password:

```bash
export FASTMAIL_DAV_PASSWORD="your-app-password"
```

Generate an app password at: https://www.fastmail.com/settings/passwords

## Output Format

Commands output JSON by default:

```json
{
  "ok": true,
  "result": [...],
  "error": null,
  "meta": {
    "rate_limit": null,
    "dry_run": false,
    "operation_id": null
  }
}
```

You can force a specific output format:

```bash
# Force JSON (even in terminal)
fastmail mail list --output json

# Force human-readable (even when piped)
fastmail mail list --output human
```

## Exit Codes

- `0`: Success
- `1`: Transient error (retry safe)
- `2`: Permanent error (do not retry)
- `3`: Safety check failed (operation rejected)

## Blob Operations

**Note:** The `fastmail blob` commands below use the JMAP Blob Management Extension (RFC 9404), which is **not supported by Fastmail**.

Fastmail does support RFC 8620 upload/download URLs internally (used for email attachments), but the CLI blob commands are not currently implemented to use those URLs.

### Not Available on Fastmail

All blob commands below require RFC 9404 support and **will not work** with Fastmail accounts:

```bash
# These commands are NOT available on Fastmail:
fastmail blob upload document.pdf --type application/pdf
fastmail blob download <BLOB_ID> output.pdf
fastmail blob info <BLOB_ID>
fastmail blob lookup <BLOB_ID> --types Email --types Mailbox
fastmail blob capability
```

These commands are included for compatibility with other JMAP providers that support RFC 9404.

## License

MIT
