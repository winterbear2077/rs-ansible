use crate::error::AnsibleError;
use crate::types::{TemplateOptions, TemplateResult};
use super::SshClient;
use std::collections::HashMap;
use tera::{Tera, Context};
use tracing::{info, debug, error};

impl SshClient {
    /// 部署模板到远程主机
    pub fn deploy_template(&self, options: &TemplateOptions) -> Result<TemplateResult, AnsibleError> {
        info!("Deploying template from '{}' to '{}'", options.src, options.dest);
        
        // 读取本地模板文件
        debug!("Reading template file: {}", options.src);
        let template_content = std::fs::read_to_string(&options.src)
            .map_err(|e| {
                error!("Failed to read template file '{}': {}", options.src, e);
                AnsibleError::FileOperationError(format!("Failed to read template file: {}", e))
            })?;
        
        // 渲染模板
        debug!("Rendering template with {} variables", options.variables.len());
        let rendered_content = self.render_template(&template_content, &options.variables)?;
        info!("Template rendered successfully, size: {} bytes", rendered_content.len());
        
        // 检查远程文件是否存在
        debug!("Checking if remote file exists: {}", options.dest);
        let remote_exists = self.check_file_exists(&options.dest)?;
        let mut changed = false;
        let mut diff = None;
        
        if remote_exists {
            debug!("Remote file exists, comparing content");
            // 获取远程文件内容
            let remote_content = self.read_remote_file(&options.dest)?;
            
            // 比较内容
            if remote_content != rendered_content {
                info!("Content differs, file will be updated");
                changed = true;
                diff = Some(self.generate_diff(&remote_content, &rendered_content));
                
                // 如果需要备份
                if options.backup {
                    info!("Creating backup of existing file");
                    self.backup_file(&options.dest)?;
                }
            } else {
                debug!("Content is identical, no changes needed");
            }
        } else {
            info!("Remote file does not exist, will be created");
            changed = true;
        }
        
        // 如果有变更，写入新内容
        if changed {
            info!("Deploying changed content to remote host");
            // 创建临时文件
            let temp_path = format!("/tmp/rs_ansible_template_{}.tmp", chrono::Utc::now().timestamp());
            let local_temp = format!("/tmp/rs_ansible_local_template_{}.tmp", chrono::Utc::now().timestamp());
            
            // 写入本地临时文件
            debug!("Writing content to local temp file: {}", local_temp);
            std::fs::write(&local_temp, &rendered_content)
                .map_err(|e| {
                    error!("Failed to write temp file: {}", e);
                    AnsibleError::FileOperationError(format!("Failed to write temp file: {}", e))
                })?;
            
            // 上传到远程临时位置
            debug!("Uploading to remote temp location: {}", temp_path);
            self.upload_file(&local_temp, &temp_path)?;
            
            // 如果提供了验证命令，先验证
            if let Some(ref validate_cmd) = options.validate {
                info!("Validating template with command: {}", validate_cmd);
                let validation_cmd = validate_cmd.replace("%s", &temp_path);
                let result = self.execute_command(&validation_cmd)?;
                
                if result.exit_code != 0 {
                    error!("Template validation failed: {}", result.stderr);
                    // 清理临时文件
                    let _ = self.execute_command(&format!("rm -f {}", temp_path));
                    let _ = std::fs::remove_file(&local_temp);
                    
                    return Err(AnsibleError::ValidationError(format!(
                        "Template validation failed: {}", result.stderr
                    )));
                }
                info!("Template validation passed");
            }
            
            // 确保目标目录存在
            if let Some(parent) = std::path::Path::new(&options.dest).parent() {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() {
                    debug!("Creating parent directory: {}", parent_str);
                    let mkdir_cmd = format!("mkdir -p '{}'", parent_str);
                    self.execute_command(&mkdir_cmd)?;
                }
            }
            
            // 移动到目标位置
            debug!("Moving file to destination: {}", options.dest);
            let mv_cmd = format!("mv '{}' '{}'", temp_path, options.dest);
            let result = self.execute_command(&mv_cmd)?;
            
            if result.exit_code != 0 {
                error!("Failed to move file to destination: {}", result.stderr);
                let _ = std::fs::remove_file(&local_temp);
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to move file to destination: {}", result.stderr
                )));
            }
            
            // 设置文件权限和所有权
            debug!("Setting file attributes (mode, owner, group)");
            self.set_file_attributes(&options.dest, options)?;
            
            // 清理本地临时文件
            let _ = std::fs::remove_file(&local_temp);
            info!("Template deployed successfully to {}", options.dest);
        } else {
            info!("Template at {} is already up to date", options.dest);
        }
        
        Ok(TemplateResult {
            success: true,
            changed,
            message: if changed {
                format!("Template deployed to {}", options.dest)
            } else {
                format!("Template at {} is already up to date", options.dest)
            },
            diff,
        })
    }

    /// 渲染模板（使用 Tera 模板引擎）
    fn render_template(&self, template: &str, variables: &HashMap<String, String>) -> Result<String, AnsibleError> {
        debug!("Creating Tera template engine instance");
        // 创建 Tera 实例
        let mut tera = Tera::default();
        
        // 添加模板字符串
        debug!("Parsing template, size: {} bytes", template.len());
        tera.add_raw_template("template", template)
            .map_err(|e| {
                error!("Failed to parse template: {}", e);
                AnsibleError::TemplateError(format!("Failed to parse template: {}", e))
            })?;
        
        // 创建上下文并添加变量
        debug!("Adding {} variables to template context", variables.len());
        let mut context = Context::new();
        for (key, value) in variables {
            context.insert(key, value);
        }
        
        // 渲染模板
        debug!("Rendering template with Tera engine");
        tera.render("template", &context)
            .map_err(|e| {
                error!("Failed to render template: {}", e);
                AnsibleError::TemplateError(format!("Failed to render template: {}", e))
            })
    }

    /// 检查远程文件是否存在
    fn check_file_exists(&self, path: &str) -> Result<bool, AnsibleError> {
        let cmd = format!("test -f '{}' && echo 'exists' || echo 'not exists'", path);
        let result = self.execute_command(&cmd)?;
        Ok(result.stdout.trim() == "exists")
    }

    /// 读取远程文件内容
    fn read_remote_file(&self, path: &str) -> Result<String, AnsibleError> {
        let cmd = format!("cat '{}'", path);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            return Err(AnsibleError::FileOperationError(format!(
                "Failed to read remote file: {}", result.stderr
            )));
        }
        
        Ok(result.stdout)
    }

    /// 生成文件差异
    fn generate_diff(&self, old_content: &str, new_content: &str) -> String {
        // 简单的行差异显示
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();
        
        let mut diff = String::new();
        diff.push_str("--- old\n");
        diff.push_str("+++ new\n");
        
        let max_lines = old_lines.len().max(new_lines.len());
        for i in 0..max_lines {
            let old_line = old_lines.get(i).unwrap_or(&"");
            let new_line = new_lines.get(i).unwrap_or(&"");
            
            if old_line != new_line {
                if !old_line.is_empty() {
                    diff.push_str(&format!("- {}\n", old_line));
                }
                if !new_line.is_empty() {
                    diff.push_str(&format!("+ {}\n", new_line));
                }
            }
        }
        
        diff
    }

    /// 备份文件
    fn backup_file(&self, path: &str) -> Result<(), AnsibleError> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = format!("{}.{}.backup", path, timestamp);
        
        info!("Creating backup: {} -> {}", path, backup_path);
        let cmd = format!("cp '{}' '{}'", path, backup_path);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            error!("Failed to backup file: {}", result.stderr);
            return Err(AnsibleError::FileOperationError(format!(
                "Failed to backup file: {}", result.stderr
            )));
        }
        
        info!("Backup created successfully: {}", backup_path);
        Ok(())
    }

    /// 上传文件到远程
    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<(), AnsibleError> {
        let mut remote_file = self.session.scp_send(
            std::path::Path::new(remote_path),
            0o644,
            std::fs::metadata(local_path)
                .map_err(|e| AnsibleError::FileOperationError(format!("Failed to get file metadata: {}", e)))?
                .len(),
            None,
        )?;

        let mut local_file = std::fs::File::open(local_path)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to open local file: {}", e)))?;

        std::io::copy(&mut local_file, &mut remote_file)
            .map_err(|e| AnsibleError::FileOperationError(format!("Failed to copy file: {}", e)))?;

        Ok(())
    }

    /// 设置文件属性（权限、所有者、组）
    fn set_file_attributes(&self, path: &str, options: &TemplateOptions) -> Result<(), AnsibleError> {
        // 设置权限
        if let Some(ref mode) = options.mode {
            debug!("Setting file mode: {} for {}", mode, path);
            let cmd = format!("chmod {} '{}'", mode, path);
            let result = self.execute_command(&cmd)?;
            if result.exit_code != 0 {
                error!("Failed to set file mode: {}", result.stderr);
                return Err(AnsibleError::FileOperationError(format!(
                    "Failed to set file mode: {}", result.stderr
                )));
            }
        }
        
        // 设置所有者和组
        if options.owner.is_some() || options.group.is_some() {
            let owner = options.owner.as_deref().unwrap_or("");
            let group = options.group.as_deref().unwrap_or("");
            
            let chown_target = if !owner.is_empty() && !group.is_empty() {
                format!("{}:{}", owner, group)
            } else if !owner.is_empty() {
                owner.to_string()
            } else {
                format!(":{}", group)
            };
            
            if !chown_target.is_empty() && chown_target != ":" {
                debug!("Setting file ownership: {} for {}", chown_target, path);
                let cmd = format!("chown {} '{}'", chown_target, path);
                let result = self.execute_command(&cmd)?;
                if result.exit_code != 0 {
                    error!("Failed to set file ownership: {}", result.stderr);
                    return Err(AnsibleError::FileOperationError(format!(
                        "Failed to set file ownership: {}", result.stderr
                    )));
                }
            }
        }
        
        Ok(())
    }
}
