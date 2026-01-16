# Setup Command and Human-Readable Output Design

## Overview

Two usability improvements to make fastmail-cli more approachable:

1. **Interactive setup command** (`fastmail setup`) to guide users through credential configuration
2. **Human-readable output by default** with `--output json` flag for automation

Both changes follow the GitHub CLI (`gh`) user experience model.

## Goals

- Lower friction for new users getting started
- Provide clear, actionable error messages
- Maintain agent/automation friendliness with JSON output
- TTY-aware: pretty output in terminals, JSON when piped

## Setup Command

### Behavior

Interactive command that guides users through configuration:

```
$ fastmail setup
→ Prompts for Fastmail API token (hidden input)
→ Validates token with live API call
→ Saves to config file with baked-in DAV endpoints
→ Confirms success with next steps
```

### Config File

**Location:** `~/.config/fastmail-cli/config.toml` (XDG standard)

**Format:**

```toml
[auth]
token = "your-api-token"

[dav]
caldav_url = "https://dav.fastmail.com/"
carddav_url = "https://dav.fastmail.com/"
```

### Error Handling

| Scenario | Message | Exit Code |
|----------|---------|-----------|
| Network error | "Couldn't reach Fastmail. Check your connection." | 1 |
| Invalid token | "API token rejected. Visit https://app.fastmail.com/settings/security/integrations" | 2 |
| Write permission denied | "Couldn't write config file. Check permissions." | 2 |
| User cancels | (silent exit) | 130 |

### Success Message

```
✓ Credentials saved!

Try: fastmail mail list
```

## Output Formatting

### TTY Detection

- Use `atty` crate to detect if stdout is a terminal
- Default: human-readable in TTY, JSON when piped
- `--output json`: force JSON even in terminal
- `--output human`: force human output even when piped

### Human Output Styles

| Command Type | Style | Example |
|--------------|-------|---------|
| Lists | Table | `ID │ Subject │ From` |
| Single items | Pretty sections | `Subject: ...` `From: ...` |
| Mutations | Success message | `✓ Created masked email: user@fastmail.com` |
| Errors | Minimal, clear | `Error: API token invalid (run 'fastmail setup')` |

### Styling

- Green ✓ for success
- Yellow for warnings
- Red for errors
- Bold for headers/keys
- Use `console` or `termcolor` crate (TTY-aware, no ANSI when piped)

## Architecture

### New Components

**`fastmail-cli/src/output/formatter.rs`**

```rust
pub enum OutputFormat {
    Auto,   // Detect from TTY
    Json,   // Force JSON
    Human,  // Force human-readable
}

pub trait Formattable {
    fn to_json(&self) -> String;
    fn to_human(&self) -> String;
}

pub fn format_output<T: Formattable>(data: &T, format: OutputFormat) -> String;
```

**`fastmail-cli/src/commands/setup.rs`**

Interactive setup command using `dialoguer` crate for prompts.

### Updated Components

- `fastmail-cli/src/commands/mod.rs`: Register setup command
- `fastmail-cli/src/main.rs`: Add `--output` flag to CLI args
- All existing commands: Implement `Formattable` trait

## Dependencies

| Crate | Purpose |
|-------|---------|
| `atty` | TTY detection |
| `dialoguer` | Interactive prompts |
| `console` or `termcolor` | Terminal styling |
| `wiremock` | Mock HTTP server for testing (dev) |
| `dirs` | XDG config directory |

## Testing

### Unit Tests

- Formatter output (both JSON and human variants)
- Config file read/write
- Error message formatting

### Integration Tests

- Setup with mock API server
- Token validation success/failure paths
- Network failure handling

### No Manual Testing

All tests automated via `cargo test`. TTY detection logic is simple enough to unit test, and we trust `atty` for actual terminal detection.

## Data Flow: Setup

```
user runs: fastmail setup
           ↓
prompt: "Enter your Fastmail API token"
           ↓
[hidden input as user types]
           ↓
API call: JMAP echo or lightweight request
           ↓
    ┌──────┴──────┐
    │             │
success       failure
    │             │
save to       show error
config        message,
file          exit 2
    │
success message:
"✓ Credentials saved!
  Try: fastmail mail list"
```

## Data Flow: Output Formatting

```
command executes
    ↓
constructs result object
    ↓
parses --output flag (default: Auto)
    ↓
calls format_output(result, format)
    ↓
    ┌──────┴──────┐
    │             │
TTY detected    No TTY
or --human    or --json
    │             │
to_human()    to_json()
    ↓             ↓
styled/tables  JSON string
    │             │
writes to stdout
```

## Exit Codes (Unchanged)

- `0`: Success
- `1`: Transient error (retry safe)
- `2`: Permanent error (do not retry)
- `3`: Safety check failed (operation rejected)
- `130`: User cancelled (SIGINT)
