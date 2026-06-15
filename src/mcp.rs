use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::tools::Tool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

struct McpConnection {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpConnection {
    async fn spawn(cfg: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&cfg.command);
        cmd.args(&cfg.args)
            .envs(&cfg.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .with_context(|| format!("启动 MCP 服务器 {} 失败", cfg.name))?;
        let stdin = child.stdin.take().context("无法打开 MCP stdin")?;
        let stdout = child.stdout.take().context("无法打开 MCP stdout")?;
        let mut conn = Self {
            _child: child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        };
        conn.initialize().await?;
        Ok(conn)
    }

    async fn initialize(&mut self) -> Result<()> {
        let _: Value = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "onemini-cli", "version": "0.1.0" }
                }),
            )
            .await?;
        self.notify("notifications/initialized", json!({})).await?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let line = format!("{req}\n");
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        loop {
            let mut buf = String::new();
            self.stdout.read_line(&mut buf).await?;
            if buf.trim().is_empty() {
                continue;
            }
            let resp: Value = serde_json::from_str(buf.trim())
                .context("解析 MCP 响应失败")?;
            if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(err) = resp.get("error") {
                    anyhow::bail!("MCP 错误: {err}");
                }
                return Ok(resp["result"].clone());
            }
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let line = format!("{req}\n");
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn list_tools(&mut self) -> Result<Vec<McpToolInfo>> {
        let result = self.request("tools/list", json!({})).await?;
        let tools = result["tools"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for t in tools {
            out.push(McpToolInfo {
                name: t["name"].as_str().unwrap_or("unknown").to_string(),
                description: t["description"].as_str().unwrap_or("").to_string(),
                input_schema: t["inputSchema"].clone(),
            });
        }
        Ok(out)
    }

    async fn call_tool(&mut self, name: &str, args: Value) -> Result<String> {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": args }),
            )
            .await?;
        if let Some(content) = result["content"].as_array() {
            let text: Vec<String> = content
                .iter()
                .filter_map(|c| c["text"].as_str().map(str::to_string))
                .collect();
            return Ok(text.join("\n"));
        }
        Ok(result.to_string())
    }
}

#[derive(Clone)]
struct McpToolInfo {
    name: String,
    description: String,
    input_schema: Value,
}

pub struct McpTool {
    info: McpToolInfo,
    connection: Arc<Mutex<McpConnection>>,
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        // mcp__server__toolname
        &self.info.name
    }

    fn description(&self) -> &str {
        &self.info.description
    }

    fn parameters_schema(&self) -> Value {
        if self.info.input_schema.is_object() {
            self.info.input_schema.clone()
        } else {
            json!({ "type": "object", "properties": {} })
        }
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let mut conn = self.connection.lock().await;
        conn.call_tool(&self.info.name, args).await
    }
}

pub struct McpRegistry {
    tools: Vec<Arc<dyn Tool>>,
}

impl McpRegistry {
    pub async fn connect_all(configs: &[McpServerConfig]) -> Result<Self> {
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
        for cfg in configs {
            match McpConnection::spawn(cfg).await {
                Ok(mut conn) => {
                    match conn.list_tools().await {
                        Ok(listed) => {
                            let conn = Arc::new(Mutex::new(conn));
                            for info in listed {
                                let tool_name = format!("mcp_{}_{}", cfg.name, info.name);
                                let mut info = info;
                                info.name = tool_name;
                                tools.push(Arc::new(McpTool {
                                    info,
                                    connection: conn.clone(),
                                }));
                            }
                        }
                        Err(e) => eprintln!("MCP {} list_tools 失败: {e}", cfg.name),
                    }
                }
                Err(e) => eprintln!("MCP {} 连接失败: {e}", cfg.name),
            }
        }
        Ok(Self { tools })
    }

    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }
}
