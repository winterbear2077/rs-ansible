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

/// 生成本地临时文件路径
/// 
/// # 参数
/// - `prefix`: 临时文件前缀，例如 "rs_ansible_template"
/// 
/// # 示例
/// ```
/// let temp_path = generate_local_temp_path("rs_ansible_template");
/// // 返回类似: "/tmp/rs_ansible_template_1732492800.123456789.987654321.tmp"
/// ```
pub fn generate_local_temp_path(prefix: &str) -> String {
    format!("/tmp/{}_{}.tmp", prefix, generate_temp_suffix())
}

/// 生成远程临时文件路径
/// 
/// # 参数
/// - `base_path`: 基础路径（目标文件路径）
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
        assert!(path.starts_with("/tmp/test_prefix_"));
        assert!(path.ends_with(".tmp"));
    }

    #[test]
    fn test_remote_temp_path_format() {
        let base = "/etc/config.conf";
        let path = generate_remote_temp_path(base);
        assert!(path.starts_with("/etc/config.conf.tmp."));
    }
}
