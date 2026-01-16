# fastmail-cli

A command-line interface for Fastmail, designed for automation and AI agents.

## Features

- **Email operations**: list, read, delete
- **Masked Email**: full support for Fastmail's masked email feature
- **Agent-first**: JSON output, exit codes, and safety mechanisms
- **Safety-first**: `--force`, `--confirm`, whitelist, and dry-run modes

## Installation

```bash
cargo install --path .
```

## Configuration

Set your Fastmail API token:

```bash
export FASTMAIL_TOKEN="your-token-here"
```

Get a token at: https://app.fastmail.com/settings/security/integrations

## Usage

### List emails

```bash
fastmail mail list --limit 10
```

### Read an email

```bash
fastmail mail read <email-id>
```

### Delete emails (with safety checks)

```bash
# Preview what would be deleted
fastmail mail delete <id> --force --confirm "delete-<id>" --dry-run

# Actually delete
fastmail mail delete <id> --force --confirm "delete-<id>"
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

### Whitelist (for send safety)

```bash
fastmail config allow-recipient add team@company.com
fastmail config allow-recipient list
fastmail config allow-recipient remove team@company.com
```

## Output Format

All commands output JSON:

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

## Exit Codes

- `0`: Success
- `1`: Transient error (retry safe)
- `2`: Permanent error (do not retry)
- `3`: Safety check failed (operation rejected)

## Blob Operations (JMAP RFC 9404)

Check if your account supports Blob operations:

```sh
fastmail blob capability
```

Upload a file as a blob:

```sh
fastmail blob upload document.pdf --type application/pdf
```

Download blob content:

```sh
fastmail blob download <BLOB_ID> output.pdf
```

Get blob metadata:

```sh
fastmail blob info <BLOB_ID>
```

Look up which objects reference a blob:

```sh
fastmail blob lookup <BLOB_ID> --types Email --types Mailbox
```

## License

MIT OR Apache-2.0
