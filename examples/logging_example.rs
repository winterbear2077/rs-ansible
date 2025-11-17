use rs_ansible::{AnsibleManager, UserOptions, UserState, TemplateOptions, HostConfig};
use std::collections::HashMap;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化 tracing 日志系统
    // 可以通过环境变量 RUST_LOG 控制日志级别
    // 例如: RUST_LOG=debug cargo run --example logging_example
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    println!("=== RS-Ansible 日志示例 ===\n");

    let mut manager = AnsibleManager::new();
    
    // 添加主机
    let host_config = HostConfig {
        hostname: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        password: Some("password".to_string()),
        private_key_path: None,
        passphrase: None,
    };
    manager.add_host("web-server".to_string(), host_config);

    println!("\n--- 示例 1: 用户管理 ---");
    println!("此操作将显示详细的用户管理日志");
    
    // 创建用户选项（仅用于演示，不会实际执行）
    let _user_options = UserOptions {
        name: "deploy".to_string(),
        state: UserState::Present,
        password: None,
        shell: Some("/bin/bash".to_string()),
        home: Some("/home/deploy".to_string()),
        group: None,
        groups: Some(vec!["sudo".to_string()]),
        uid: None,
        comment: Some("Deployment user".to_string()),
        system: false,
        create_home: true,
        expires: None,
    };

    println!("日志级别:");
    println!("  - INFO:  显示主要操作步骤");
    println!("  - DEBUG: 显示详细的命令和内部操作");
    println!("  - ERROR: 显示错误信息");

    // 注意: 实际使用时需要连接到真实主机
    // let result = manager.manage_user_on_hosts(&user_options, &["web-server".to_string()]).await;

    println!("\n--- 示例 2: 模板部署 ---");
    println!("此操作将显示详细的模板渲染和部署日志");

    let mut variables = HashMap::new();
    variables.insert("app_name".to_string(), serde_json::Value::String("MyApp".to_string()));
    variables.insert("port".to_string(), serde_json::Value::Number(serde_json::Number::from(8080)));
    variables.insert("environment".to_string(), serde_json::Value::String("production".to_string()));

    let _template_options = TemplateOptions {
        src: "examples/app.conf.tera".to_string(),
        dest: "/etc/myapp/config.conf".to_string(),
        variables,
        mode: Some("0644".to_string()),
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        backup: true,
        validate: None,
    };

    // 注意: 实际使用时需要连接到真实主机
    // let result = manager.deploy_template_to_hosts(&template_options, &["web-server".to_string()]).await;

    println!("\n日志功能说明:");
    println!("1. 通过 RUST_LOG 环境变量控制日志级别");
    println!("   - RUST_LOG=info     # 仅显示重要信息");
    println!("   - RUST_LOG=debug    # 显示详细调试信息");
    println!("   - RUST_LOG=trace    # 显示所有跟踪信息");
    println!("\n2. 用户管理日志:");
    println!("   - 用户存在性检查");
    println!("   - 用户创建/修改/删除操作");
    println!("   - 命令执行详情");
    println!("   - 错误信息");
    println!("\n3. 模板部署日志:");
    println!("   - 模板文件读取");
    println!("   - Tera 引擎渲染过程");
    println!("   - 远程文件比较");
    println!("   - 文件备份");
    println!("   - 验证命令执行");
    println!("   - 权限设置");
    println!("   - 所有者/组设置");

    println!("\n运行示例:");
    println!("  RUST_LOG=info cargo run --example logging_example");
    println!("  RUST_LOG=debug cargo run --example logging_example");

    Ok(())
}
