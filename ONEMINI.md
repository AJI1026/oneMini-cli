# OneMini CLI 项目说明

本文件会被 CLI 自动加载为项目上下文（类似 Claude Code 的 CLAUDE.md）。

OneMini-CLI 是用 Rust 实现的终端 AI 编程助手，提供交互式 REPL 对话、Agent 工具循环（read / write / edit / grep / glob / bash）及 OpenAI 兼容 API（DeepSeek、OpenAI、硅基流动等）。

```bash
cargo build --release
cargo run -- chat
```
