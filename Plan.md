# Fastmail CLI 实施计划

## 项目概述

构建一个现代化的 Rust 命令行工具，专门为 Fastmail 优化，专注于脚本化、自动化和快速操作。

### 项目定位

**与 meli 的差异化**：
- **meli**: TUI 邮件客户端（类似 Alpine/Mutt，交互式终端界面）
- **fastmail-cli**: CLI 工具（类似 git/gh，命令行批处理、自动化、CI/CD 集成）

### 核心价值主张

1. ✅ **快速命令执行** - 无需启动 TUI，一行命令完成操作
2. ✅ **脚本友好** - JSON 输出，适合管道处理和自动化
3. ✅ **Fastmail 特有功能** - Masked Email 深度集成
4. ✅ **CI/CD 就绪** - 易于集成到 DevOps 流程
5. ✅ **开发者体验** - 清晰的 CLI 设计，优秀的错误提示

---

## Phase 1: 项目初始化 (1-2 天)

### 1.1 项目设置

```bash
# 创建项目
cargo new fastmail-cli --name fastmail
cd fastmail-cli

# 添加基础依赖
cargo add clap --features derive,env,color
cargo add tokio --features full
cargo add serde --features derive
cargo add serde_json
cargo add reqwest --features json,rustls-tls
cargo add anyhow
cargo add thiserror
cargo add directories
cargo add toml
cargo add colored
```

### 1.2 项目结构

```
fastmail-cli/
├── Cargo.toml
├── README.md
├── LICENSE
├── Plan.md                 # 本文件
├── src/
│   ├── main.rs              # CLI 入口
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs          # Clap 参数定义
│   │   └── commands/        # 子命令处理
│   │       ├── mod.rs
│   │       ├── mail.rs
│   │       ├── masked.rs
│   │       ├── config.rs
│   │       └── auth.rs
│   ├── jmap/
│   │   ├── mod.rs
│   │   ├── client.rs        # JMAP 客户端封装
│   │   └── types.rs         # 类型转换
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── token.rs         # API token 管理
│   │   └── oauth.rs         # OAuth 2.0 实现
│   ├── config/
│   │   ├── mod.rs
│   │   └── storage.rs       # 配置文件管理
│   └── output/
│       ├── mod.rs
│       ├── table.rs         # 表格输出
│       └── json.rs         # JSON 输出
└── tests/
    ├── integration/
    └── fixtures/
```

### 1.3 基础 CLI 框架

**src/main.rs**
```rust
use clap::{Parser, Subcommand};
use fastmail::cli::commands::*;

#[derive(Parser)]
#[command(name = "fastmail")]
#[command(about = "A modern CLI tool for Fastmail", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Email operations
    Mail(MailArgs),
    /// Masked email management
    Masked(MaskedArgs),
    /// Configuration management
    Config(ConfigArgs),
    /// Authentication setup
    Auth(AuthArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Mail(args) => commands::mail::execute(args).await?,
        Commands::Masked(args) => commands::masked::execute(args).await?,
        Commands::Config(args) => commands::config::execute(args).await?,
        Commands::Auth(args) => commands::auth::execute(args).await?,
    }
    Ok(())
}
```

**Deliverable**: ✅ 可编译的 CLI 框架，支持基本子命令结构

---

## Phase 2: 核心基础设施 (2-3 天)

### 2.1 配置管理

**功能需求**：
- 配置文件存储：`~/.config/fastmail-cli/config.toml`
- API token 安全存储
- 支持多个账户配置
- 环境变量支持

**config.toml 结构**：
```toml
[default_account]
email = "user@fastmail.com"

[accounts."user@fastmail.com"]
email = "user@fastmail.com"
auth_type = "token"  # or "oauth"

[accounts."user@fastmail.com".token]
access_token = "fmu1-xxxxxxxxxxxxxxxxxxxxxxx"
# 或

[accounts."user@fastmail.com".oauth]
client_id = "your-client-id"
access_token = "..."
refresh_token = "..."
expires_at = 1704326400

[display]
theme = "auto"  # auto, light, dark
output_format = "table"  # table, json, plain
```

**实现文件**：
- `src/config/storage.rs` - 读写配置文件
- `src/cli/commands/config.rs` - config 子命令

**Deliverable**: ✅ 配置系统，支持 `fastmail config` 命令

### 2.2 认证系统

**优先级 1**: API Token（快速实现）
```rust
// src/auth/token.rs
pub struct TokenAuth {
    email: String,
    api_token: String,
}

impl TokenAuth {
    pub fn new(email: String, api_token: String) -> Self {
        Self { email, api_token }
    }

    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.api_token)
    }
}
```

**优先级 2**: OAuth 2.0（完整支持）
```rust
// src/auth/oauth.rs
pub struct OAuthFlow {
    client_id: String,
    redirect_uri: String,
}

impl OAuthFlow {
    pub async fn authorize(&self) -> anyhow::Result<(String, String)> {
        // 生成 PKCE code_challenge
        // 构建 authorization URL
        // 返回 URL 和 state
    }

    pub async fn exchange_code(&self, code: String, verifier: String)
        -> anyhow::Result<OAuthTokens>
    {
        // 交换 authorization code 为 tokens
    }

    pub async fn refresh_token(&self, refresh_token: String)
        -> anyhow::Result<OAuthTokens>
    {
        // 使用 refresh token 获取新的 access token
    }
}
```

**Deliverable**: ✅ 认证系统，支持 `fastmail auth login` 命令

### 2.3 JMAP 客户端封装

基于 `jmap-client` 库：

```rust
// src/jmap/client.rs
use jmap_client::client::Client;

pub struct FastmailClient {
    inner: Client,
    account_id: String,
}

impl FastmailClient {
    pub async fn new_with_token(
        email: &str,
        api_token: &str,
    ) -> anyhow::Result<Self> {
        let inner = Client::new()
            .credentials((email, api_token))
            .connect("https://api.fastmail.com/jmap/session")
            .await?;

        // 从 session 获取 account_id
        let account_id = inner.session().await?
            .accounts
            .values()
            .next()
            .ok_or_else(|| anyhow!("No account found"))?
            .clone();

        Ok(Self { inner, account_id })
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    // 代理常用操作
    pub async fn list_mailboxes(&self) -> anyhow::Result<Vec<Mailbox>> {
        let result = self.inner.mailbox_get(None::<String>).await?;
        Ok(result.take_list())
    }

    pub async fn query_emails(
        &self,
        mailbox_id: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<Email>> {
        let ids = self.inner
            .email_query(
                jmap_client::email::query::Filter::in_mailbox(mailbox_id),
                [jmap_client::email::query::Comparator::from()].into(),
            )
            .await?
            .take_ids();

        let emails = self.inner
            .email_get(&ids[..limit.min(ids.len())])
            .await?
            .take_list();

        Ok(emails)
    }
}
```

**Deliverable**: ✅ JMAP 客户端，支持基本邮箱操作

---

## Phase 3: MVP 邮件功能 (3-5 天)

### 3.1 邮件列表

**命令**：
```bash
fastmail mail list [OPTIONS]

Options:
  -m, --mailbox <NAME>     邮箱名称 [default: INBOX]
  -l, --limit <NUM>        最多显示数量 [default: 20]
  -f, --folder <NAME>      按文件夹筛选
  -o, --format <FORMAT>   输出格式 [default: table]
                           [possible: table, json, plain]
```

**实现**：
```rust
// src/cli/commands/mail.rs
pub async fn execute(args: MailListArgs) -> anyhow::Result<()> {
    let client = get_client().await?;
    let mailboxes = client.list_mailboxes().await?;

    let mailbox = find_mailbox(&mailboxes, args.mailbox)?;

    let emails = client.query_emails(&mailbox.id, args.limit).await?;

    match args.format {
        OutputFormat::Table => output::table::print_emails(&emails),
        OutputFormat::Json => output::json::print_emails(&emails),
        OutputFormat::Plain => output::plain::print_emails(&emails),
    }

    Ok(())
}
```

**示例输出**：
```bash
$ fastmail mail list --limit 5

┌──────────────────────┬────────────────────┬────────────────────┬────────┐
│ ID                 │ From            │ Subject          │ Date   │
├──────────────────────┼────────────────────┼────────────────────┼────────┤
│ A1B2C3D4E5F6      │ alice@example.com │ Project Update  │ Today  │
│ B2C3D4E5F6A7      │ bob@company.com  │ Meeting Tomorrow │ Today  │
└──────────────────────┴────────────────────┴────────────────────┴────────┘
```

**Deliverable**: ✅ `fastmail mail list` 命令

### 3.2 邮件读取

**命令**：
```bash
fastmail mail read <ID> [OPTIONS]

Options:
  -f, --format <FORMAT>   输出格式 [default: auto]
                           [possible: auto, text, html, both]
  -o, --output <FILE>    输出到文件
```

**实现**：
```rust
pub async fn execute(args: MailReadArgs) -> anyhow::Result<()> {
    let client = get_client().await?;
    let email = client.email_get(&args.id).await?;

    match args.format {
        OutputFormat::Auto => {
            if email.has_text_body() {
                println!("{}", email.text_body()?);
            } else {
                println!("{}", email.html_body()?);
            }
        }
        OutputFormat::Text => println!("{}", email.text_body()?),
        OutputFormat::Html => println!("{}", email.html_body()?),
        OutputFormat::Both => {
            println!("=== TEXT ===\n{}", email.text_body()?);
            println!("\n=== HTML ===\n{}", email.html_body()?);
        }
    }

    Ok(())
}
```

**Deliverable**: ✅ `fastmail mail read <id>` 命令

### 3.3 邮件发送

**命令**：
```bash
fastmail mail send [OPTIONS]

Required:
  -t, --to <EMAIL>       收件人（可重复）
  -s, --subject <SUBJECT> 主题

Optional:
  -b, --body <TEXT>       邮件正文（或 -f）
  -f, --file <FILE>       从文件读取正文
  -c, --cc <EMAIL>        抄送（可重复）
  -b, --bcc <EMAIL>       密送（可重复）
  -a, --attach <FILE>    附件（可重复）
  -e, --editor            使用编辑器编写
```

**实现**：
```rust
pub async fn execute(args: MailSendArgs) -> anyhow::Result<()> {
    let client = get_client().await?;

    let body = if let Some(file) = args.file {
        tokio::fs::read_to_string(file).await?
    } else if let Some(text) = args.body {
        text
    } else if args.editor {
        open_editor_and_get_body()?
    } else {
        return Err(anyhow!("Either --body, --file, or --editor required"));
    };

    let result = client.send_email(
        &args.to,
        args.subject.as_deref().unwrap_or(&String::new()),
        &body,
        args.cc.as_deref(),
        args.bcc.as_deref(),
        args.attachments.as_deref(),
    ).await?;

    println!("✓ Email sent successfully");
    println!("  Message ID: {}", result.message_id);

    Ok(())
}
```

**Deliverable**: ✅ `fastmail mail send` 命令

### 3.4 邮件搜索

**命令**：
```bash
fastmail mail search <QUERY> [OPTIONS]

Options:
  -f, --from <EMAIL>      按发件人筛选
  -t, --to <EMAIL>        按收件人筛选
  -d, --date <DATE>       日期范围（格式：YYYY-MM-DD 或 YYYY-MM-DD..YYYY-MM-DD）
  -a, --attachments        只显示有附件的
  -l, --limit <NUM>       最多结果数
  -m, --mailbox <NAME>     指定邮箱搜索
```

**实现**：
```rust
pub async fn execute(args: MailSearchArgs) -> anyhow::Result<()> {
    let client = get_client().await?;

    let mut filter = vec![];

    if let Some(from) = &args.from {
        filter.push(jmap_client::email::query::Filter::from(from));
    }

    if let Some(subject) = &args.query {
        filter.push(jmap_client::email::query::Filter::subject(subject));
    }

    if args.attachments {
        filter.push(jmap_client::email::query::Filter::has_attachment());
    }

    let combined = jmap_client::email::query::Filter::and(filter);

    let emails = client.search_emails(combined, args.limit).await?;

    output::table::print_emails(&emails);

    Ok(())
}
```

**Deliverable**: ✅ `fastmail mail search` 命令

---

## Phase 4: Masked Email 功能 (2-3 天) ⭐ 差异化点

### 4.1 创建 Masked Email

**命令**：
```bash
fastmail masked create <DOMAIN> [OPTIONS]

Required:
  -d, --domain <DOMAIN>    目标域名（如：example.com）

Optional:
  -p, --prefix <TEXT>      邮箱前缀（可选）
  -d, --description <TEXT>  描述信息
  -c, --copy              自动复制到剪贴板
```

**实现**：
```rust
// src/cli/commands/masked.rs
pub async fn execute_create(args: MaskedCreateArgs) -> anyhow::Result<()> {
    let client = get_client().await?;

    let masked = client.create_masked_email(
        &args.domain,
        args.prefix.as_deref(),
        args.description.as_deref().unwrap_or(&String::new()),
    ).await?;

    println!("✓ Masked email created:");
    println!("  Email: {}", masked.email);
    println!("  For: {}", masked.for_domain);
    println!("  State: {}", masked.state);

    if args.copy {
        copy_to_clipboard(&masked.email)?;
        println!("  Copied to clipboard!");
    }

    Ok(())
}
```

### 4.2 列出 Masked Emails

**命令**：
```bash
fastmail masked list [OPTIONS]

Options:
  -f, --filter <DOMAIN>    按域名筛选
  -s, --state <STATE>      按状态筛选 [pending|enabled|disabled]
  -o, --format <FORMAT>   输出格式 [default: table]
```

**实现**：
```rust
pub async fn execute_list(args: MaskedListArgs) -> anyhow::Result<()> {
    let client = get_client().await?;
    let masked_emails = client.list_masked_emails().await?;

    let filtered = if let Some(domain) = args.filter {
        masked_emails.into_iter()
            .filter(|m| m.for_domain.contains(&domain))
            .collect()
    } else if let Some(state) = args.state {
        masked_emails.into_iter()
            .filter(|m| m.state == state)
            .collect()
    } else {
        masked_emails
    };

    output::table::print_masked_emails(&filtered);

    Ok(())
}
```

**示例输出**：
```bash
$ fastmail masked list

┌─────────────────────────┬──────────────┬──────────────────┬────────┬────────────┐
│ Email              │ Domain       │ Description    │ State  │ Last Used  │
├─────────────────────────┼──────────────┼──────────────────┼────────┼────────────┤
│ abc123@fastmail.com │ example.com  │ Shopping site  │ enabled │ Today      │
│ def456@fastmail.com │ news.com     │ Newsletter     │ enabled │ Yesterday  │
│ ghi789@fastmail.com │ social.com    │ Social app     │ disabled│ 2 days ago  │
└─────────────────────────┴──────────────┴──────────────────┴────────┴────────────┘
```

### 4.3 管理 Masked Email

**命令**：
```bash
fastmail masked <ACTION> <EMAIL>

Actions:
  enable    启用邮箱
  disable   禁用邮箱（发送到垃圾箱）
  delete    删除邮箱（退回邮件）
```

**实现**：
```rust
pub async fn execute_manage(args: MaskedManageArgs) -> anyhow::Result<()> {
    let client = get_client().await?;

    match args.action {
        MaskedAction::Enable => {
            client.enable_masked_email(&args.email).await?;
            println!("✓ Masked email {} enabled", args.email);
        }
        MaskedAction::Disable => {
            client.disable_masked_email(&args.email).await?;
            println!("✓ Masked email {} disabled", args.email);
        }
        MaskedAction::Delete => {
            client.delete_masked_email(&args.email).await?;
            println!("✓ Masked email {} deleted", args.email);
        }
    }

    Ok(())
}
```

**Deliverable**: ✅ 完整的 Masked Email 管理功能

---

## Phase 5: 增强功能 (5-7 天)

### 5.1 邮件操作

**添加命令**：
```bash
# 回复邮件
fastmail mail reply <ID> [OPTIONS]

# 转发邮件
fastmail mail forward <ID> [OPTIONS]

# 删除邮件
fastmail mail delete <ID>...

# 移动邮件
fastmail mail move <ID>... --to <MAILBOX>

# 标记邮件
fastmail mail mark <ID>... --<read|unread|flagged|unflagged>
```

### 5.2 批量操作

```bash
# 批量删除
fastmail mail delete <ID> <ID> ...

# 批量移动
fastmail mail move <ID>... --to <MAILBOX>

# 批量标记
fastmail mail mark <ID>... --read
```

### 5.3 输出格式增强

**JSON 输出**：
```bash
$ fastmail mail list --format json | jq '.[] | select(.unread)'

{
  "id": "A1B2C3D4E5F6",
  "from": {"email": "alice@example.com", "name": "Alice"},
  "subject": "Project Update",
  "preview": "Just wanted to update you...",
  "date": "2026-01-15T10:30:00Z",
  "unread": true,
  "flagged": false,
  "attachments": []
}
```

**Plain 输出**：
```bash
$ fastmail mail read A1B2C3D4E5F6 --format plain

From: alice@example.com
To: user@fastmail.com
Subject: Project Update
Date: 2026-01-15 10:30:00

Just wanted to update you on the project progress...
```

### 5.4 自动化友好功能

```bash
# 快速检查未读邮件数量
fastmail mail --unread-count
5

# 快速获取收件箱预览
fastmail inbox --preview

# 监控新邮件（轮询模式）
fastmail watch --interval 60 --command "notify-send 'New mail: {}'"
```

**Deliverable**: ✅ 增强的邮件操作和自动化功能

---

## Phase 6: 测试与文档 (3-4 天)

### 6.1 单元测试

```rust
// tests/unit/jmap_client_test.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jmap_client_creation() {
        let client = FastmailClient::new_with_token(
            "test@fastmail.com",
            "test-token",
        ).await;

        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_masked_email_creation() {
        // Mock JMAP responses
        // Test creation logic
    }
}
```

### 6.2 集成测试

```rust
// tests/integration/masked_email_test.rs
#[tokio::test]
#[ignore] // 需要真实的 Fastmail 账户
async fn test_create_and_list_masked_email() {
    let client = setup_test_client().await?;

    let created = client.create_masked_email(
        "test.example.com",
        None,
        "Test integration",
    ).await?;

    let list = client.list_masked_emails().await?;

    assert!(list.iter().any(|m| m.email == created.email));

    cleanup_test_data(client, &created.email).await?;
}
```

### 6.3 文档

**README.md 结构**：
```markdown
# fastmail-cli

A modern CLI tool for Fastmail with full support for JMAP and Masked Email.

## Features

- 📧 Full email management (list, read, send, search)
- 🎭 Masked Email integration
- 🚀 Fast, scriptable commands
- 🔒 Secure authentication (API token & OAuth 2.0)
- 📊 Multiple output formats (table, JSON, plain)
- 🤖 CI/CD ready

## Installation

```bash
cargo install fastmail-cli
```

## Quick Start

1. Authenticate:
```bash
fastmail auth login
# 或
fastmail config set token <your-api-token>
```

2. List emails:
```bash
fastmail mail list
```

3. Create masked email:
```bash
fastmail masked create example.com --description "Sign up"
```

## Usage

See [USAGE.md](USAGE.md) for detailed command reference.

## Examples

### Automation

Send alerts from scripts:
```bash
#!/bin/bash
if [ -f "error.log" ]; then
    fastmail mail send \
        --to dev-team@company.com \
        --subject "Error in production" \
        --body "$(cat error.log)"
fi
```

### CI/CD Integration

```yaml
- name: Notify deployment
  run: |
    fastmail mail send \
        --to stakeholders@company.com \
        --subject "Deployed to ${{ environment }}" \
        --body "Commit: ${{ github.sha }}"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0
```

**USAGE.md** - 详细命令参考
**CONTRIBUTING.md** - 贡献指南
**CHANGELOG.md** - 变更记录

**Deliverable**: ✅ 完整的文档和测试覆盖

---

## Phase 7: 发布与推广 (1-2 天)

### 7.1 发布准备

```bash
# 版本号
export VERSION="0.1.0"

# 更新版本号
sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml
sed -i "s/^## \\[Unreleased]/## [$VERSION] - $(date +%Y-%m-%d)/" CHANGELOG.md

# 构建 release
cargo build --release

# 创建 tag
git tag "v$VERSION"
git push origin "v$VERSION"
```

### 7.2 发布到 crates.io

```bash
# 登录 crates.io
cargo login

# 发布
cargo publish
```

### 7.3 GitHub Release

- 创建 GitHub Release
- 附上编译好的二进制文件（Linux, macOS, Windows）
- 写详细的 Release Notes

### 7.4 推广

**目标社区**：
- r/rust
- r/fastmail
- r/selfhosted
- r/commandline
- Hacker News
- Lobste.rs

**推文示例**：
```markdown
# Title
fastmail-cli: A Modern Rust CLI for Fastmail with Masked Email Support

# Content

I've built a command-line tool for Fastmail that focuses on:
- Quick, scriptable operations (not a TUI)
- Full Masked Email integration
- JSON output for automation
- CI/CD ready

Installation: `cargo install fastmail-cli`

GitHub: https://github.com/yourusername/fastmail-cli
```

**Deliverable**: ✅ 发布到 crates.io，推文到社区

---

## 时间线总览

```infographic
sequence-timeline-simple
data
  title 开发时间线
  items
    - label Week 1
      desc 项目初始化、配置、认证
    - label Week 2-3
      desc JMAP 客户端、MVP 邮件功能
    - label Week 4
      desc Masked Email 功能
    - label Week 5-6
      desc 增强功能、批理操作
    - label Week 7
      desc 测试、文档、发布
```

---

## 成功指标

### 量化指标

- [ ] 📦 发布到 crates.io
- [ ] ⭐ 50+ GitHub stars
- [ ] 📥 10+ contributors
- [ ] 💬 100+ issues/discussions
- [ ] 📚 文档覆盖 80%+ 的命令

### 质量指标

- [ ] 所有命令都有 `--help` 文档
- [ ] 所有错误都有清晰的错误消息
- [ ] 90%+ 的代码有单元测试
- [ ] 集成测试覆盖主要流程
- [ ] 响应时间 <24h（GitHub issues）

---

## 风险与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| JMAP API 变化 | 中 | 高 | 保持与 Fastmail 开发者联系，及时更新 |
| OAuth 实现复杂 | 低 | 高 | MVP 使用 API token，OAuth 作为 v2 |
| 与 meli 功能重叠 | 高 | 中 | 强调 CLI vs TUI 的差异，专注自动化 |
| 社区采纳度低 | 中 | 中 | 积极推文，完善文档，响应反馈 |
| 性能问题 | 低 | 中 | 异步操作，批量处理，缓存优化 |

---

## 后续路线图

### v0.2.0
- [ ] OAuth 2.0 完整支持
- [ ] 邮件附件管理
- [ ] 多账户支持（账户切换）
- [ ] 配置文件加密

### v0.3.0
- [ ] 联系人管理（CardDAV/JMAP）
- [ ] 日历集成（CalDAV/JMAP）
- [ ] 邮件标签管理
- [ ] 邮件模板

### v0.4.0
- [ ] 实时邮件推送
- [ ] 离线模式（本地缓存）
- [ ] 插件系统
- [ ] GUI 前端（可选）

### v1.0.0
- [ ] 企业版特性
- [ ] 云服务集成
- [ ] 移动端应用（可能）

---

## 总结

这个计划专注于：

1. ✅ **差异化** - 与 meli 的 TUI 定位明显区分
2. ✅ **实用性** - 解决真实的自动化和脚本需求
3. ✅ **可实现性** - 基于成熟的 `jmap-client` 库
4. ✅ **市场清晰** - 专注于 CLI/自动化细分市场
5. ✅ **可扩展性** - 为未来功能预留架构空间

**开始执行**：
```bash
# 从 Phase 1 开始
cargo new fastmail-cli --name fastmail
cd fastmail-cli
```

祝你好运！🚀
