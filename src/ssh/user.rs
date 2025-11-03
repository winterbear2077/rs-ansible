use crate::error::AnsibleError;
use crate::types::{UserOptions, UserResult, UserInfo, UserState};
use super::SshClient;
use tracing::{info, debug, error};

impl SshClient {
    /// 管理用户（创建、修改或删除）
    pub fn manage_user(&self, options: &UserOptions) -> Result<UserResult, AnsibleError> {
        info!("Managing user '{}' with state: {:?}", options.name, options.state);
        match options.state {
            UserState::Present => self.ensure_user_present(options),
            UserState::Absent => self.ensure_user_absent(options),
        }
    }

    /// 确保用户存在
    fn ensure_user_present(&self, options: &UserOptions) -> Result<UserResult, AnsibleError> {
        debug!("Checking if user '{}' exists", options.name);
        // 检查用户是否已存在
        let user_exists = self.check_user_exists(&options.name)?;
        
        if user_exists {
            info!("User '{}' already exists, checking if update is needed", options.name);
            // 用户已存在，检查是否需要修改
            let current_info = self.get_user_info(&options.name)?;
            let needs_update = self.check_user_needs_update(&current_info, options);
            
            if needs_update {
                info!("User '{}' needs update, modifying user", options.name);
                // 修改用户
                self.modify_user(options)?;
                let updated_info = self.get_user_info(&options.name)?;
                info!("User '{}' updated successfully", options.name);
                Ok(UserResult {
                    success: true,
                    changed: true,
                    message: format!("User '{}' updated successfully", options.name),
                    user_info: Some(updated_info),
                })
            } else {
                debug!("User '{}' already has correct configuration", options.name);
                // 用户已存在且无需修改
                Ok(UserResult {
                    success: true,
                    changed: false,
                    message: format!("User '{}' already exists with correct configuration", options.name),
                    user_info: Some(current_info),
                })
            }
        } else {
            info!("User '{}' does not exist, creating new user", options.name);
            // 创建新用户
            self.create_user(options)?;
            let user_info = self.get_user_info(&options.name)?;
            info!("User '{}' created successfully", options.name);
            Ok(UserResult {
                success: true,
                changed: true,
                message: format!("User '{}' created successfully", options.name),
                user_info: Some(user_info),
            })
        }
    }

    /// 确保用户不存在
    fn ensure_user_absent(&self, options: &UserOptions) -> Result<UserResult, AnsibleError> {
        debug!("Checking if user '{}' exists for removal", options.name);
        let user_exists = self.check_user_exists(&options.name)?;
        
        if user_exists {
            info!("Deleting user '{}'", options.name);
            // 删除用户
            self.delete_user(&options.name)?;
            info!("User '{}' removed successfully", options.name);
            Ok(UserResult {
                success: true,
                changed: true,
                message: format!("User '{}' removed successfully", options.name),
                user_info: None,
            })
        } else {
            debug!("User '{}' does not exist, no action needed", options.name);
            // 用户不存在，无需操作
            Ok(UserResult {
                success: true,
                changed: false,
                message: format!("User '{}' does not exist", options.name),
                user_info: None,
            })
        }
    }

    /// 检查用户是否存在
    fn check_user_exists(&self, username: &str) -> Result<bool, AnsibleError> {
        let cmd = format!("id -u {} > /dev/null 2>&1 && echo 'exists' || echo 'not exists'", username);
        let result = self.execute_command(&cmd)?;
        Ok(result.stdout.trim() == "exists")
    }

    /// 获取用户信息
    fn get_user_info(&self, username: &str) -> Result<UserInfo, AnsibleError> {
        let cmd = format!("getent passwd {}", username);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            return Err(AnsibleError::CommandError(format!(
                "Failed to get user info: {}", result.stderr
            )));
        }

        // 解析 passwd 格式: username:x:uid:gid:comment:home:shell
        let parts: Vec<&str> = result.stdout.trim().split(':').collect();
        if parts.len() < 7 {
            return Err(AnsibleError::CommandError(
                "Invalid passwd format".to_string()
            ));
        }

        Ok(UserInfo {
            name: parts[0].to_string(),
            uid: parts[2].parse().map_err(|e| AnsibleError::CommandError(format!("Invalid UID: {}", e)))?,
            gid: parts[3].parse().map_err(|e| AnsibleError::CommandError(format!("Invalid GID: {}", e)))?,
            comment: parts[4].to_string(),
            home: parts[5].to_string(),
            shell: parts[6].to_string(),
        })
    }

    /// 检查用户是否需要更新
    fn check_user_needs_update(&self, current: &UserInfo, options: &UserOptions) -> bool {
        // 检查各项配置是否匹配
        if let Some(uid) = options.uid {
            if current.uid != uid {
                return true;
            }
        }
        
        if let Some(ref home) = options.home {
            if &current.home != home {
                return true;
            }
        }
        
        if let Some(ref shell) = options.shell {
            if &current.shell != shell {
                return true;
            }
        }
        
        if let Some(ref comment) = options.comment {
            if &current.comment != comment {
                return true;
            }
        }
        
        // 检查组成员关系（简化版）
        if options.group.is_some() || options.groups.is_some() {
            // 这里可以添加更详细的组检查逻辑
            // 为了简化，假设总是需要更新
            return true;
        }
        
        false
    }

    /// 创建用户
    fn create_user(&self, options: &UserOptions) -> Result<(), AnsibleError> {
        debug!("Building useradd command for user '{}'", options.name);
        let mut cmd = String::from("useradd");
        
        if let Some(uid) = options.uid {
            cmd.push_str(&format!(" -u {}", uid));
        }
        
        if let Some(ref group) = options.group {
            cmd.push_str(&format!(" -g {}", group));
        }
        
        if let Some(ref groups) = options.groups {
            cmd.push_str(&format!(" -G {}", groups.join(",")));
        }
        
        if let Some(ref home) = options.home {
            cmd.push_str(&format!(" -d {}", home));
        }
        
        if let Some(ref shell) = options.shell {
            cmd.push_str(&format!(" -s {}", shell));
        }
        
        if let Some(ref comment) = options.comment {
            cmd.push_str(&format!(" -c '{}'", comment.replace("'", "'\\''")));
        }
        
        if options.create_home {
            cmd.push_str(" -m");
        } else {
            cmd.push_str(" -M");
        }
        
        if options.system {
            cmd.push_str(" -r");
        }
        
        if let Some(ref expires) = options.expires {
            cmd.push_str(&format!(" -e {}", expires));
        }
        
        cmd.push_str(&format!(" {}", options.name));
        
        debug!("Executing useradd command: {}", cmd);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            error!("Failed to create user '{}': {}", options.name, result.stderr);
            return Err(AnsibleError::CommandError(format!(
                "Failed to create user: {}", result.stderr
            )));
        }
        
        // 如果提供了密码，设置密码
        if let Some(ref password) = options.password {
            debug!("Setting password for user '{}'", options.name);
            self.set_user_password(&options.name, password)?;
        }
        
        Ok(())
    }

    /// 修改用户
    fn modify_user(&self, options: &UserOptions) -> Result<(), AnsibleError> {
        debug!("Building usermod command for user '{}'", options.name);
        let mut cmd = String::from("usermod");
        
        if let Some(uid) = options.uid {
            cmd.push_str(&format!(" -u {}", uid));
        }
        
        if let Some(ref group) = options.group {
            cmd.push_str(&format!(" -g {}", group));
        }
        
        if let Some(ref groups) = options.groups {
            cmd.push_str(&format!(" -G {}", groups.join(",")));
        }
        
        if let Some(ref home) = options.home {
            cmd.push_str(&format!(" -d {}", home));
        }
        
        if let Some(ref shell) = options.shell {
            cmd.push_str(&format!(" -s {}", shell));
        }
        
        if let Some(ref comment) = options.comment {
            cmd.push_str(&format!(" -c '{}'", comment.replace("'", "'\\''")));
        }
        
        if let Some(ref expires) = options.expires {
            cmd.push_str(&format!(" -e {}", expires));
        }
        
        cmd.push_str(&format!(" {}", options.name));
        
        debug!("Executing usermod command: {}", cmd);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            error!("Failed to modify user '{}': {}", options.name, result.stderr);
            return Err(AnsibleError::CommandError(format!(
                "Failed to modify user: {}", result.stderr
            )));
        }
        
        // 如果提供了密码，设置密码
        if let Some(ref password) = options.password {
            debug!("Updating password for user '{}'", options.name);
            self.set_user_password(&options.name, password)?;
        }
        
        Ok(())
    }

    /// 删除用户
    fn delete_user(&self, username: &str) -> Result<(), AnsibleError> {
        debug!("Executing userdel command for user '{}'", username);
        let cmd = format!("userdel -r {}", username);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            error!("Failed to delete user '{}': {}", username, result.stderr);
            return Err(AnsibleError::CommandError(format!(
                "Failed to delete user: {}", result.stderr
            )));
        }
        
        Ok(())
    }

    /// 设置用户密码
    fn set_user_password(&self, username: &str, encrypted_password: &str) -> Result<(), AnsibleError> {
        // 使用 chpasswd 或 usermod -p 设置已加密的密码
        let cmd = format!("echo '{}:{}' | chpasswd -e", username, encrypted_password);
        let result = self.execute_command(&cmd)?;
        
        if result.exit_code != 0 {
            return Err(AnsibleError::CommandError(format!(
                "Failed to set user password: {}", result.stderr
            )));
        }
        
        Ok(())
    }
}
