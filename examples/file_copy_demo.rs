use rs_ansible::{AnsibleManager, FileCopyOptions, Result};
use std::fs;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化tracing日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(false)
        .init();

    setup_test_files()?;
    
    // 2. 配置SSH连接
    let mut manager = AnsibleManager::new();
    
    // 添加目标主机（请修改为您的实际SSH服务器信息）
    // manager.add_host(
    //     "test-server".to_string(),
    //     AnsibleManager::host_builder()
    //         .hostname("179.10.15.128")  // 修改为实际主机地址
    //         .port(22)
    //         .username("root")   // 修改为实际用户名
    //         .password("root")  // 或使用private_key_path
    //         // .private_key_path("/home/user/.ssh/id_rsa")
    //         .build()
    // );
    
    let hosts = [
        "179.10.18.1",
        "179.10.18.2",
        "179.10.18.3",
        "179.10.18.4",
        "179.10.18.5",
        "179.10.18.6",
        "179.10.18.7",
        "179.10.18.8",
        "179.10.18.9",
        "179.10.18.10",
    ];

    let _ = hosts.iter().for_each(|&host| {
        manager.add_host(format!("test-server-{}", host),
            AnsibleManager::host_builder()
            .hostname(host)
            .username("root")
            .password("mod.root.0815")
            .port(22)
            .build()
        );
    });

    
    // 3. 测试SSH连接
    let _ping_result = manager.ping_all().await;
    
    // 4. 基本文件复制（无选项）
    let local_file1 = "/tmp/rs_ansible_test/local/simple_config.txt";
    let remote_file1 = "/tmp/rs_ansible_test/remote/simple_config.txt";
    
    
    let _copy_result1 = manager.copy_file_to_all(local_file1, remote_file1).await;
    
    // 5. 带Hash验证的文件复制
    let local_file2 = "/tmp/rs_ansible_test/local/config_with_hash.txt";
    let remote_file2 = "/tmp/rs_ansible_test/remote/config_with_hash.txt";
    
    let hash_options = FileCopyOptions {
        verify_hash: true,
        hash_algorithm: Some("sha256".to_string()),
        mode: Some("644".to_string()),
        create_dirs: true,
        backup: false,
        ..Default::default()
    };
    
    
    let _copy_result2 = manager.copy_file_to_all_with_options(local_file2, remote_file2, &hash_options).await;
    let _copy_result3 = manager.copy_file_to_all_with_options(local_file2, remote_file2, &hash_options).await;
    
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(local_file2)?;
    file.write_all(b"\n# Modified content\n")?;
    
    let _copy_result4 = manager.copy_file_to_all_with_options(local_file2, remote_file2, &hash_options).await;
    // 8. 带完整选项的文件复制（不包含owner/group以避免权限问题）
    let local_file3 = "/tmp/rs_ansible_test/local/script.sh";
    let remote_file3 = "/tmp/rs_ansible_test/remote/script.sh";
    
    let full_options = FileCopyOptions {
        mode: Some("755".to_string()),
        verify_hash: true,
        hash_algorithm: Some("sha256".to_string()),
        backup: true,
        create_dirs: true,
        ..Default::default()
    };

    let _copy_result5 = manager.copy_file_to_all_with_options(local_file3, remote_file3, &full_options).await;
    
    // 9. 验证远程文件
    let verify_cmd = format!("ls -lh {} {} {}", remote_file1, remote_file2, remote_file3);
    
    let _verify_result = manager.execute_command_all(&verify_cmd).await;
    
    Ok(())
}

/// 创建本地测试文件
fn setup_test_files() -> Result<()> {
    // 创建测试目录
    fs::create_dir_all("/tmp/rs_ansible_test/local")?;
    
    // 1. 简单配置文件
    let simple_config = "/tmp/rs_ansible_test/local/simple_config.txt";
    let mut file = fs::File::create(simple_config)?;
    file.write_all(b"# Simple Configuration\nserver_url=http://example.com\nport=8080\n")?;
    
    // 2. 带hash验证的配置文件
    let hash_config = "/tmp/rs_ansible_test/local/config_with_hash.txt";
    let mut file = fs::File::create(hash_config)?;
    file.write_all(b"# Configuration with Hash Verification\ndatabase_host=localhost\ndatabase_port=5432\n")?;
    
    // 3. 可执行脚本
    let script = "/tmp/rs_ansible_test/local/script.sh";
    let mut file = fs::File::create(script)?;
    file.write_all(b"#!/bin/bash\necho 'Hello from deployed script'\ndate\n")?;
    
    // 4. 较大的测试文件（用于测试传输性能）
    let large_file = "/tmp/rs_ansible_test/local/large_file.dat";
    let mut file = fs::File::create(large_file)?;
    let content = "X".repeat(1024 * 100); // 100KB
    file.write_all(content.as_bytes())?;
    
    Ok(())
}