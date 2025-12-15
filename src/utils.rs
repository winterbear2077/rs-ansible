/// 生成唯一的临时文件后缀
/// 
/// 使用纳秒级时间戳 + 随机数，确保在高并发场景下不会产生文件名冲突。
/// 
/// # 示例
/// ```
/// let suffix = generate_temp_suffix();
/// let temp_file = format!("/tmp/my_file_{}.tmp", suffix);
/// ```
pub fn generate_temp_suffix() -> String {
    let now = chrono::Utc::now();
    let timestamp = now.timestamp();
    let nanos = now.timestamp_subsec_nanos();
    let random_suffix: u32 = rand::random();
    
    format!("{}.{}.{}", timestamp, nanos, random_suffix)
}

/// 生成本地临时文件路径（支持跨平台）
/// 
/// 根据本地操作系统自动选择合适的临时目录：
/// - Windows: %TEMP% 或 C:\Windows\Temp
/// - Unix/Linux/macOS: /tmp
/// 
/// # 参数
/// - `prefix`: 临时文件前缀，例如 "rs_ansible_template"
/// 
/// # 示例
/// ```
/// let temp_path = generate_local_temp_path("rs_ansible_template");
/// // Unix: "/tmp/rs_ansible_template_1732492800.123456789.987654321.tmp"
/// // Windows: "C:\Users\Username\AppData\Local\Temp\rs_ansible_template_1732492800.123456789.987654321.tmp"
/// ```
pub fn generate_local_temp_path(prefix: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 TEMP 环境变量或默认临时目录
        let temp_dir = std::env::var("TEMP")
            .or_else(|_| std::env::var("TMP"))
            .unwrap_or_else(|_| "C:\\Windows\\Temp".to_string());
        format!("{}\\{}_{}.tmp", temp_dir, prefix, generate_temp_suffix())
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // Unix/Linux/macOS: 使用 /tmp
        format!("/tmp/{}_{}.tmp", prefix, generate_temp_suffix())
    }
}

/// 生成远程临时文件路径（仅支持 Unix/Linux 远程主机）
/// 
/// 注意：远程主机路径始终使用 Unix 格式，因为 rs-ansible 只支持 Unix/Linux 远程主机。
/// 
/// # 参数
/// - `base_path`: 基础路径（目标文件路径），必须是 Unix 格式
/// 
/// # 示例
/// ```
/// let temp_path = generate_remote_temp_path("/etc/nginx/nginx.conf");
/// // 返回类似: "/etc/nginx/nginx.conf.tmp.1732492800.123456789.987654321"
/// ```
pub fn generate_remote_temp_path(base_path: &str) -> String {
    format!("{}.tmp.{}", base_path, generate_temp_suffix())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_temp_suffix_uniqueness() {
        // 测试生成的后缀是否唯一
        let mut suffixes = HashSet::new();
        
        for _ in 0..1000 {
            let suffix = generate_temp_suffix();
            assert!(
                suffixes.insert(suffix.clone()),
                "Generated duplicate suffix: {}",
                suffix
            );
        }
    }

    #[test]
    fn test_local_temp_path_format() {
        let path = generate_local_temp_path("test_prefix");
        
        #[cfg(target_os = "windows")]
        {
            // Windows 路径应该包含反斜杠
            assert!(path.contains("\\"));
            assert!(path.contains("test_prefix_"));
            assert!(path.ends_with(".tmp"));
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // Unix 路径应该以 /tmp 开头
            assert!(path.starts_with("/tmp/test_prefix_"));
            assert!(path.ends_with(".tmp"));
        }
    }

    #[test]
    fn test_remote_temp_path_format() {
        let base = "/etc/config.conf";
        let path = generate_remote_temp_path(base);
        // 远程路径始终是 Unix 格式
        assert!(path.starts_with("/etc/config.conf.tmp."));
        assert!(!path.contains("\\"));  // 不应该包含 Windows 路径分隔符
    }
}
