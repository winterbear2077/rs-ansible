use crate::manager::*;
#[cfg(test)]
use crate::types::*;

#[test]
fn test_host_config_builder() {
    let config = AnsibleManager::host_builder()
        .hostname("test.example.com")
        .port(2222)
        .username("testuser")
        .password("testpass")
        .build();

    assert_eq!(config.hostname, "test.example.com");
    assert_eq!(config.port, 2222);
    assert_eq!(config.username, "testuser");
    assert_eq!(config.password, Some("testpass".to_string()));
}

#[test]
fn test_host_config_default() {
    let config = HostConfig::default();
    assert_eq!(config.port, 22);
    assert!(config.password.is_none());
    assert!(config.private_key_path.is_none());
}

#[test]
fn test_ansible_manager_operations() {
    let mut manager = AnsibleManager::new();

    let config = AnsibleManager::host_builder()
        .hostname("192.168.1.100")
        .username("test")
        .password("test")
        .build();

    // 测试添加主机
    manager.add_host("test_host".to_string(), config.clone());
    assert_eq!(manager.list_hosts().len(), 1);

    // 测试获取主机
    let retrieved_config = manager.get_host("test_host");
    assert!(retrieved_config.is_some());
    assert_eq!(retrieved_config.unwrap().hostname, "192.168.1.100");

    // 测试移除主机
    let removed_config = manager.remove_host("test_host");
    assert!(removed_config.is_some());
    assert_eq!(manager.list_hosts().len(), 0);
}

#[test]
fn test_command_result() {
    let result = CommandResult {
        exit_code: 0,
        stdout: "Hello World".to_string(),
        stderr: "".to_string(),
    };

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "Hello World");
    assert!(result.stderr.is_empty());
}

#[test]
fn test_batch_result() {
    let mut batch_result: BatchResult<bool> = BatchResult::new();

    batch_result.add_result("host1".to_string(), Ok(true));
    batch_result.add_result(
        "host2".to_string(),
        Err(crate::error::AnsibleError::SshConnectionError(
            "Test error".to_string(),
        )),
    );

    assert_eq!(batch_result.successful.len(), 1);
    assert_eq!(batch_result.failed.len(), 1);
    assert_eq!(batch_result.success_rate(), 0.5);
}

#[test]
fn test_system_info_serialization() {
    use std::collections::HashMap;

    let mut disk_usage = HashMap::new();
    disk_usage.insert("/".to_string(), "50%".to_string());

    let network_interfaces = vec![NetworkInterface {
        name: "eth0".to_string(),
        ip_address: "192.168.1.100".to_string(),
        mac_address: "00:11:22:33:44:55".to_string(),
    }];

    let sys_info = SystemInfo {
        hostname: "test-host".to_string(),
        os: "Linux".to_string(),
        kernel_version: "5.4.0".to_string(),
        architecture: "x86_64".to_string(),
        uptime: "up 1 day".to_string(),
        memory_total: "8G".to_string(),
        memory_free: "4G".to_string(),
        disk_usage,
        cpu_info: "Intel Core i7".to_string(),
        network_interfaces,
    };

    // 测试序列化
    let json = serde_json::to_string(&sys_info).unwrap();
    assert!(json.contains("test-host"));
    assert!(json.contains("Linux"));

    // 测试反序列化
    let deserialized: SystemInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.hostname, "test-host");
    assert_eq!(deserialized.network_interfaces.len(), 1);
}
