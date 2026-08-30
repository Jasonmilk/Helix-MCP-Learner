//! Helix-MCP-Learner CLI 入口
//!
//! 用法：
//!   mcp-learner learn --command <cmd> --args <arg1> --args <arg2> --output <dir>
//!   mcp-learner list --server-config <config.json>

use clap::{Parser, Subcommand};
use mcp_learner::{McpClient, extract_tools_with_namespace, ManifestGenerator};
use mcp_learner::post_learn::{ReviewPipeline, ReviewPipelineConfig};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "mcp-learner", version, about = "MCP Server → CI-144 工具定义的自动学习器")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 日志级别
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 学习 MCP Server，生成 Tentacle 插件 Manifest
    Learn {
        /// MCP Server 启动命令
        #[arg(long)]
        command: String,

        /// MCP Server 启动参数（可多次指定）
        #[arg(long, num_args = 0..)]
        args: Vec<String>,

        /// 输出目录
        #[arg(long, default_value = "./plugins")]
        output: PathBuf,

        /// MCP Server 名称（用于命名输出文件）
        #[arg(long)]
        name: Option<String>,
    },

    /// 列出已学习的工具（读取输出目录）
    List {
        /// 插件目录
        #[arg(long, default_value = "./plugins")]
        plugins_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(format!("mcp_learner={}", cli.log_level))
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Commands::Learn {
            command,
            args,
            output,
            name,
        } => {
            let server_name = name.unwrap_or_else(|| {
                command
                    .split('/')
                    .last()
                    .unwrap_or("mcp-server")
                    .to_string()
            });

            info!("Starting MCP Server: {} {:?}", command, args);

            // 连接 MCP Server
            let mut client = McpClient::connect(&command, &args).await?;
            info!("Connected to MCP Server");

            // 初始化
            client.initialize().await?;
            info!("MCP session initialized");

            // 获取工具列表
            let tools = client.list_tools().await?;
            info!("Discovered {} tools", tools.len());

            for tool in &tools {
                info!("  - {} (risk: {:?})", tool.name, mcp_learner::risk_rating(&tool.name));
            }

            // 提炼为 CI-144 工具定义（带命名空间前缀，符合点分命名空间规范）
            let ci144_tools = extract_tools_with_namespace(&tools, &server_name);
            info!("Extracted {} CI-144 tools", ci144_tools.len());

            // 生成 Manifest
            let generator = ManifestGenerator::new(&server_name, &command, args);
            let tool_set = generator.generate_set(&server_name, &ci144_tools);
            info!("Generated {} manifests", tool_set.tools.len());

            // L1 静态审查 + 状态迁移自动化（raw → stable/staging/rejected）
            let review_config = ReviewPipelineConfig {
                output_root: output.clone(),
                warning_to_staging: true,
                generate_report: true,
            };
            let review_pipeline = ReviewPipeline::new(review_config);
            let review_result = review_pipeline.process_and_write(&server_name, &tool_set.tools)?;

            info!(
                "Review complete: stable={}, staging={}, rejected={}",
                review_result.state_counts.get("stable").unwrap_or(&0),
                review_result.state_counts.get("staging").unwrap_or(&0),
                review_result.state_counts.get("rejected").unwrap_or(&0)
            );

            // 关闭连接
            client.close().await?;
            info!("MCP session closed");

            println!("\n✅ Learning complete!");
            println!("   Server: {}", server_name);
            println!("   Tools: {}", tools.len());
            println!("   Output: {:?}", output);
            println!("\n   Review results:");
            println!("     ✅ Stable:  {} tools (ready to use)", review_result.state_counts.get("stable").unwrap_or(&0));
            println!("     ⚠️  Staging: {} tools (warnings, need confirmation)", review_result.state_counts.get("staging").unwrap_or(&0));
            println!("     ❌ Rejected: {} tools (errors, must fix)", review_result.state_counts.get("rejected").unwrap_or(&0));
            println!("\n   Load in Tentacle:");
            println!("   tentacle --transport stdio --plugins-dir {:?}", output.join("stable"));
        }

        Commands::List { plugins_dir } => {
            info!("Listing plugins in {:?}", plugins_dir);

            if !plugins_dir.exists() {
                warn!("Plugins directory does not exist: {:?}", plugins_dir);
                println!("No plugins found. Run 'mcp-learner learn' first.");
                return Ok(());
            }

            let mut count = 0;
            for entry in std::fs::read_dir(&plugins_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json")
                    && !path.file_name().unwrap().to_str().unwrap().contains("_index")
                {
                    let content = std::fs::read_to_string(&path)?;
                    if let Ok(manifest) = serde_json::from_str::<mcp_learner::manifest::ToolManifest>(&content) {
                        println!(
                            "  {:30} {:10} {:8} {}",
                            manifest.name,
                            manifest.version,
                            format!("{:?}", manifest.security_level),
                            manifest.description
                        );
                        count += 1;
                    }
                }
            }

            if count == 0 {
                println!("No plugins found.");
            } else {
                println!("\nTotal: {} plugins", count);
            }
        }
    }

    Ok(())
}
