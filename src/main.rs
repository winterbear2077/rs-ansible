use rs_ansible::{AnsibleManager, InventoryConfig, TaskExecutor, Task, Playbook, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化tracing日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
    
    println!("=== Rust Ansible Library Advanced Demo ===\n");
    
    // 演示基本功能
    demo_basic_functionality().await?;
    
    // 演示配置文件功能
    demo_config_file_functionality().await?;
    
    // 演示任务执行器功能
    demo_task_executor_functionality().await?;
    
    Ok(())
}

async fn demo_basic_functionality() -> Result<()> {
    println!("📋 1. 基本功能演示");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建Ansible管理器
    let mut manager = AnsibleManager::new();
    
    // 添加主机配置示例
    let host1 = AnsibleManager::host_builder()
        .hostname("192.168.1.100")
        .port(22)
        .username("ubuntu")
        .password("demo_password")  // 仅为演示
        .build();
    
    let host2 = AnsibleManager::host_builder()
        .hostname("192.168.1.101")
        .port(22)
        .username("ubuntu")
        .private_key_path("/home/user/.ssh/id_rsa")
        .build();
    
    manager.add_host("web-server".to_string(), host1);
    manager.add_host("db-server".to_string(), host2);
    
    println!("✅ 已配置主机: {:?}", manager.list_hosts());
    println!();
    
    Ok(())
}

async fn demo_config_file_functionality() -> Result<()> {
    println!("📂 2. 配置文件功能演示");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建示例配置
    let mut inventory = InventoryConfig::new();
    
    // 添加主机配置
    let web_server = AnsibleManager::host_builder()
        .hostname("web1.example.com")
        .username("deploy")
        .private_key_path("/home/user/.ssh/deploy_key")
        .build();
    
    let db_server = AnsibleManager::host_builder()
        .hostname("db1.example.com")
        .username("deploy")
        .private_key_path("/home/user/.ssh/deploy_key")
        .build();
    
    inventory.hosts.insert("web1".to_string(), web_server);
    inventory.hosts.insert("db1".to_string(), db_server);
    
    // 添加主机组
    inventory.add_host_to_group("web1".to_string(), "webservers".to_string());
    inventory.add_host_to_group("db1".to_string(), "databases".to_string());
    
    // 保存配置到YAML文件
    match inventory.save_to_yaml("inventory.yml") {
        Ok(_) => println!("✅ 配置已保存到 inventory.yml"),
        Err(e) => println!("❌ 保存配置失败: {}", e),
    }
    
    // 保存配置到JSON文件
    match inventory.save_to_json("inventory.json") {
        Ok(_) => println!("✅ 配置已保存到 inventory.json"),
        Err(e) => println!("❌ 保存配置失败: {}", e),
    }
    
    println!("📊 配置统计:");
    println!("   - 主机数量: {}", inventory.hosts.len());
    println!("   - 组数量: {}", inventory.groups.len());
    for group in inventory.get_groups() {
        let hosts = inventory.get_hosts_in_group(group);
        println!("   - 组 '{}': {:?}", group, hosts);
    }
    
    println!();
    Ok(())
}

async fn demo_task_executor_functionality() -> Result<()> {
    println!("🚀 3. 任务执行器功能演示");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 创建管理器并添加一些演示主机
    let mut manager = AnsibleManager::new();
    let demo_host = AnsibleManager::host_builder()
        .hostname("demo.example.com")
        .username("demo")
        .password("demo")
        .build();
    manager.add_host("demo-host".to_string(), demo_host);
    
    // 创建任务执行器
    let _executor = TaskExecutor::new(&manager);
    
    // 创建一个示例Playbook
    let playbook = Playbook::new("系统维护任务")
        .add_task(Task::ping("连接测试").ignore_errors())
        .add_task(Task::command("获取系统信息", "uname -a"))
        .add_task(Task::command("检查磁盘使用", "df -h"))
        .add_task(Task::shell_script("系统更新检查", r#"
#!/bin/bash
echo "开始系统检查..."
echo "当前时间: $(date)"
echo "系统负载: $(uptime)"
echo "内存使用:"
free -h
echo "检查完成!"
"#))
        .add_task(Task::system_info("收集详细系统信息"));
    
    // 保存Playbook到文件
    match playbook.save_to_file("maintenance_playbook.yml") {
        Ok(_) => println!("✅ Playbook已保存到 maintenance_playbook.yml"),
        Err(e) => println!("❌ 保存Playbook失败: {}", e),
    }
    
    // 展示Playbook内容
    println!("📋 Playbook '{}' 包含 {} 个任务:", playbook.name, playbook.tasks.len());
    for (i, task) in playbook.tasks.iter().enumerate() {
        println!("   {}. {}", i + 1, task.name);
    }
    
    println!("\n💡 注意: 由于演示环境限制，实际的SSH连接可能会失败。");
    println!("   在真实环境中，请:");
    println!("   - 配置正确的主机地址和认证信息");
    println!("   - 确保目标主机可达且SSH服务正常");
    println!("   - 使用SSH密钥认证替代密码认证");
    
    // 创建单独的任务演示
    println!("\n🔧 任务构建器演示:");
    let sample_tasks = vec![
        Task::command("检查服务状态", "systemctl status nginx"),
        Task::copy_file("部署配置文件", "/local/config.conf", "/remote/config.conf")
            .on_hosts(vec!["web1".to_string(), "web2".to_string()]),
        Task::shell_script("备份脚本", "tar -czf /backup/$(date +%Y%m%d).tar.gz /var/www/")
            .ignore_errors(),
    ];
    
    for task in &sample_tasks {
        println!("   - 任务: {} (忽略错误: {})", task.name, task.ignore_errors);
        if let Some(ref hosts) = task.hosts {
            println!("     目标主机: {:?}", hosts);
        } else {
            println!("     目标主机: 所有主机");
        }
    }
    
    println!();
    Ok(())
}
