use rs_ansible::{AnsibleManager, Result};
use tracing::{info, debug, warn, error, trace};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化tracing日志，支持通过环境变量控制日志级别
    // 使用方法: RUST_LOG=debug cargo run --example tracing_demo
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("=== RS-Ansible Tracing 日志演示 ===");
    
    // 演示不同级别的日志
    demo_log_levels();
    
    // 演示实际操作中的日志
    demo_ssh_operations().await?;
    
    info!("演示完成!");
    
    Ok(())
}

fn demo_log_levels() {
    info!("📋 步骤1: 演示不同的日志级别");
    
    trace!("TRACE: 最详细的日志，通常用于追踪程序执行流程");
    debug!("DEBUG: 调试信息，用于开发阶段");
    info!("INFO: 一般信息，记录重要的业务逻辑");
    warn!("WARN: 警告信息，可能存在问题但不影响运行");
    error!("ERROR: 错误信息，程序遇到了问题");
    
    info!("提示: 使用环境变量控制日志级别");
    info!("  RUST_LOG=trace  - 显示所有日志");
    info!("  RUST_LOG=debug  - 显示debug及以上级别");
    info!("  RUST_LOG=info   - 显示info及以上级别（默认）");
    info!("  RUST_LOG=warn   - 仅显示warn和error");
    info!("  RUST_LOG=error  - 仅显示error");
    info!("");
    info!("  示例: RUST_LOG=debug cargo run --example tracing_demo");
    info!("");
}

async fn demo_ssh_operations() -> Result<()> {
    info!("📋 步骤2: 演示SSH操作中的日志");
    
    let mut manager = AnsibleManager::new();
    
    debug!("创建测试主机配置...");
    
    // 添加一些测试主机（这些主机可能不存在，仅用于演示日志）
    let test_hosts = vec![
        ("web1", "192.168.1.10"),
        ("web2", "192.168.1.11"),
        ("db1", "192.168.1.20"),
    ];
    
    for (name, ip) in test_hosts {
        debug!("添加主机: {} ({})", name, ip);
        manager.add_host(
            name.to_string(),
            AnsibleManager::host_builder()
                .hostname(ip)
                .username("deploy")
                .password("demo_password")
                .build()
        );
    }
    
    info!("已添加 {} 台主机", manager.list_hosts().len());
    
    info!("尝试连接主机...");
    warn!("注意: 这些是演示主机，可能无法实际连接");
    
    let ping_result = manager.ping_all().await;
    
    if ping_result.success_rate() > 0.0 {
        info!("✓ 成功连接 {} 台主机", ping_result.successful.len());
        for host in &ping_result.successful {
            debug!("  {} - 连接成功", host);
        }
    }
    
    if !ping_result.failed.is_empty() {
        warn!("✗ {} 台主机连接失败", ping_result.failed.len());
        for host in &ping_result.failed {
            error!("  {} - 连接失败", host);
        }
    }
    
    info!("连接测试完成，成功率: {:.1}%", ping_result.success_rate() * 100.0);
    
    Ok(())
}
