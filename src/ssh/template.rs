use crate::error::AnsibleError;
use crate::types::{TemplateOptions, TemplateResult, FileCopyOptions};
use crate::utils::{generate_local_temp_path, generate_remote_temp_path};
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
        let mut rendered_content = self.render_template(&template_content, &options.variables)?;
        
        // 确保渲染后的内容使用 Unix 换行符 (\n)，避免在 Windows 上生成 \r\n 导致执行失败
        if rendered_content.contains('\r') {
            debug!("Removing CR characters from rendered template content");
            rendered_content = rendered_content.replace('\r', "");
        }
        
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
                    self.backup_remote_file(&options.dest)?;
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
            // 创建本地临时文件（使用统一的工具函数生成唯一路径）
            let local_temp = generate_local_temp_path("rs_ansible_template");
            
            // 写入渲染后的内容到本地临时文件
            debug!("Writing rendered content to local temp file: {}", local_temp);
            std::fs::write(&local_temp, &rendered_content)
                .map_err(|e| {
                    error!("Failed to write temp file: {}", e);
                    AnsibleError::FileOperationError(format!("Failed to write temp file: {}", e))
                })?;
            
            // 如果提供了验证命令，需要先上传到临时位置验证
            if let Some(ref validate_cmd) = options.validate {
                info!("Validating template before deployment");
                let temp_remote = generate_remote_temp_path("/tmp/rs_ansible_validate");
                
                // ✅ 使用 file_transfer 的方法上传到临时位置（带 SHA256 验证）
                let temp_options = FileCopyOptions {
                    mode: Some("644".to_string()),
                    owner: None,
                    group: None,
                    backup: false,
                    create_dirs: true,
                };
                self.copy_file_to_remote_with_options(&local_temp, &temp_remote, &temp_options)?;
                
                // 执行验证命令
                let validation_cmd = validate_cmd.replace("%s", &temp_remote);
                let result = self.execute_command(&validation_cmd)?;
                
                // 清理远程临时文件
                let _ = self.execute_command(&format!("rm -f '{}'", temp_remote));
                
                if result.exit_code != 0 {
                    error!("Template validation failed: {}", result.stderr);
                    let _ = std::fs::remove_file(&local_temp);
                    return Err(AnsibleError::ValidationError(format!(
                        "Template validation failed: {}", result.stderr
                    )));
                }
                info!("Template validation passed");
            }
            
            // ✅ 使用 file_transfer 的方法上传文件（自动带 SHA256 验证、幂等性检查、原子性保证）
            info!("Uploading rendered template to remote host with integrity verification");
            let file_options = FileCopyOptions {
                mode: options.mode.clone(),
                owner: options.owner.clone(),
                group: options.group.clone(),
                backup: false, // 已经在前面处理过备份
                create_dirs: true, // 自动创建目标目录
            };
            
            let transfer_result = self.copy_file_to_remote_with_options(&local_temp, &options.dest, &file_options)?;
            info!("Template uploaded: {}", transfer_result.message);
            
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
    fn render_template(&self, template: &str, variables: &HashMap<String, serde_json::Value>) -> Result<String, AnsibleError> {
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
            // ✅ 直接插入 serde_json::Value，Tera 的 Context 支持任意可序列化的值
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

    /// 备份远程文件
    fn backup_remote_file(&self, path: &str) -> Result<(), AnsibleError> {
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
}
