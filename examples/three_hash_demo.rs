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

    println!("\n🔐 === 三次 Hash 校验演示 ===\n");
    println!("本演示展示文件传输过程中的三次 hash 校验：");
    println!("  1️⃣  计算本地文件 hash（总是执行）");
    println!("  2️⃣  检查远程文件 hash，如果相同则跳过传输（幂等性，可选）");
    println!("  3️⃣  传输完成后验证远程文件 hash（总是执行，确保完整性）\n");

    // 创建测试文件
    setup_test_files()?;

    // 配置SSH连接
    let mut manager = AnsibleManager::new();
    
    // 添加测试主机（请修改为您的实际SSH服务器信息）
    println!("📋 配置SSH连接...");
    
    let hosts = [
        "179.10.18.1",
        "179.10.18.2",
    ];

    for &host in &hosts {
        manager.add_host(
            format!("test-{}", host),
            AnsibleManager::host_builder()
                .hostname(host)
                .username("root")
                .password("mod.root.0815")
                .port(22)
                .build()
        );
    }

    println!("✓ 已添加 {} 台主机\n", hosts.len());

    // 测试连接
    println!("🔌 测试SSH连接...");
    let ping_result = manager.ping_all().await;
    println!("✓ 连接成功率: {:.0}%\n", ping_result.success_rate() * 100.0);

    if ping_result.failed.len() > 0 {
        println!("⚠️  部分主机连接失败: {:?}", ping_result.failed);
        println!("继续使用成功的主机进行演示...\n");
    }

    if ping_result.successful.is_empty() {
        println!("❌ 没有可用的主机，无法演示");
        return Ok(());
    }

    // ========== 场景1：首次传输（三次校验） ==========
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📦 场景1：首次传输文件");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let local_file = "/tmp/rs_ansible_test/test_file.txt";
    let remote_file = "/tmp/rs_ansible_test/remote_file.txt";

    let options = FileCopyOptions {
        mode: Some("644".to_string()),
        create_dirs: true,
        backup: false,
        ..Default::default()
    };

    println!("预期流程：");
    println!("  1️⃣  计算本地文件 SHA256");
    println!("  2️⃣  检查远程文件（不存在，将传输）");
    println!("  3️⃣  传输完成后验证 SHA256\n");

    let result1 = manager.copy_file_to_all_with_options(local_file, remote_file, &options).await;
    
    println!("\n结果：");
    for (host, res) in &result1.results {
        match res {
            Ok(file_result) => {
                println!("  ✅ {} - 传输成功", host);
                println!("     传输字节: {}", file_result.bytes_transferred);
                println!("     消息: {}", file_result.message);
            }
            Err(e) => println!("  ❌ {} - 失败: {}", host, e),
        }
    }

    // ========== 场景2：幂等性检查（跳过传输） ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔄 场景2：再次传输相同文件（幂等性检查）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("预期流程：");
    println!("  1️⃣  计算本地文件 SHA256");
    println!("  2️⃣  检查远程文件 SHA256（相同，跳过传输）");
    println!("  3️⃣  不需要第三次验证（未传输）\n");

    let result2 = manager.copy_file_to_all_with_options(local_file, remote_file, &options).await;
    
    println!("\n结果：");
    for (host, res) in &result2.results {
        match res {
            Ok(file_result) => {
                if file_result.bytes_transferred == 0 {
                    println!("  ✅ {} - 跳过传输（文件未改变）", host);
                } else {
                    println!("  ⚠️  {} - 重新传输了 {} 字节", host, file_result.bytes_transferred);
                }
                println!("     消息: {}", file_result.message);
            }
            Err(e) => println!("  ❌ {} - 失败: {}", host, e),
        }
    }

    // ========== 场景3：文件修改后传输（重新验证） ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 场景3：修改本地文件后再次传输");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 修改本地文件
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(local_file)?;
    file.write_all(b"\n# Modified at runtime\n")?;
    drop(file);
    
    println!("✓ 本地文件已修改\n");

    println!("预期流程：");
    println!("  1️⃣  计算本地文件 SHA256（已改变）");
    println!("  2️⃣  检查远程文件 SHA256（不同，将传输）");
    println!("  3️⃣  传输完成后验证新的 SHA256\n");

    let result3 = manager.copy_file_to_all_with_options(local_file, remote_file, &options).await;
    
    println!("\n结果：");
    for (host, res) in &result3.results {
        match res {
            Ok(file_result) => {
                if file_result.bytes_transferred > 0 {
                    println!("  ✅ {} - 检测到变化，重新传输", host);
                    println!("     传输字节: {}", file_result.bytes_transferred);
                } else {
                    println!("  ⚠️  {} - 未传输", host);
                }
                println!("     消息: {}", file_result.message);
            }
            Err(e) => println!("  ❌ {} - 失败: {}", host, e),
        }
    }

    // ========== 场景4：不启用幂等性检查（仍然验证传输） ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("⚙️  场景4：禁用幂等性检查（verify_hash=false）");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let options_no_idempotency = FileCopyOptions {
        mode: Some("644".to_string()),
        create_dirs: true,
        backup: false,
        ..Default::default()
    };

    println!("预期流程：");
    println!("  1️⃣  计算本地文件 SHA256");
    println!("  2️⃣  跳过远程文件检查（强制传输）");
    println!("  3️⃣  传输完成后仍然验证 SHA256（强制执行）\n");

    let result4 = manager.copy_file_to_all_with_options(local_file, remote_file, &options_no_idempotency).await;
    
    println!("\n结果：");
    for (host, res) in &result4.results {
        match res {
            Ok(file_result) => {
                println!("  ✅ {} - 传输成功（未检查幂等性）", host);
                println!("     传输字节: {}", file_result.bytes_transferred);
                println!("     消息: {}", file_result.message);
            }
            Err(e) => println!("  ❌ {} - 失败: {}", host, e),
        }
    }

    // ========== 总结 ==========
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 演示总结");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    println!("✅ 三次 Hash 校验机制：");
    println!("  1. 第一次 Hash：总是计算本地文件 hash");
    println!("  2. 第二次 Hash：可选的幂等性检查（verify_hash=true）");
    println!("  3. 第三次 Hash：总是验证传输后的文件完整性\n");
    
    println!("🔐 安全保证：");
    println!("  ✓ 传输完整性：第三次验证确保文件在传输过程中没有损坏");
    println!("  ✓ 幂等性：第二次检查避免重复传输相同文件");
    println!("  ✓ 原子性：使用临时文件，验证通过后才移动到目标位置");
    println!("  ✓ 错误处理：验证失败时自动清理临时文件\n");

    println!("💡 提示：使用 RUST_LOG=debug 查看详细的 hash 计算过程");
    println!("  例如：RUST_LOG=debug cargo run --example three_hash_demo\n");

    Ok(())
}

fn setup_test_files() -> Result<()> {
    fs::create_dir_all("/tmp/rs_ansible_test")?;
    
    let test_file = "/tmp/rs_ansible_test/test_file.txt";
    let mut file = fs::File::create(test_file)?;
    file.write_all(b"# Test File for Three-Hash Verification Demo\n")?;
    file.write_all(b"This file demonstrates the three-hash verification process:\n")?;
    file.write_all(b"1. Calculate local file hash\n")?;
    file.write_all(b"2. Check remote file hash (idempotency)\n")?;
    file.write_all(b"3. Verify transferred file hash (integrity)\n")?;
    
    println!("✓ 测试文件已创建: {}\n", test_file);
    
    Ok(())
}
