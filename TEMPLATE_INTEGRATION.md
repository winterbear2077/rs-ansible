# 模板模块集成完成

## 完成的工作

### 1. 添加 Tera 依赖
- 在 `Cargo.toml` 中添加了 `tera = "1.19"` 依赖

### 2. 更新模板实现
- 重写了 `src/ssh/template.rs` 中的 `render_template` 方法
- 从简单的字符串替换改为使用 Tera 模板引擎
- 支持 Tera 的所有功能：变量、条件、循环、过滤器、宏等

### 3. 创建示例模板文件
- `examples/nginx.conf.tera` - Nginx 配置模板
- `examples/app.conf.tera` - 应用配置模板
- `examples/systemd.service.tera` - Systemd 服务文件模板

### 4. 创建示例代码
- `examples/template_example.rs` - 完整的模板部署示例
- `examples/tera_features.rs` - Tera 功能演示

### 5. 编写文档
- `TERA_GUIDE.md` - 详细的 Tera 使用指南
- 更新 `README.md` - 添加模板功能说明

## Tera 模板引擎优势

相比之前的简单字符串替换，Tera 提供了：

### 1. 丰富的语法支持
```jinja2
# 变量和过滤器
{{ name | upper }}
{{ port | default(value="8080") }}

# 条件语句
{% if environment == "production" %}
  ...
{% endif %}

# 循环
{% for server in servers %}
  upstream {{ server.host }}:{{ server.port }}
{% endfor %}
```

### 2. 内置过滤器
- 字符串: `upper`, `lower`, `capitalize`, `trim`, `truncate`, `replace`
- 数值: `round`, `abs`
- 数组: `first`, `last`, `join`, `length`, `sort`, `reverse`
- 日期: `date`

### 3. 模板继承
```jinja2
{% extends "base.conf" %}
{% block content %}
  自定义内容
{% endblock %}
```

### 4. 宏（可重用片段）
```jinja2
{% macro render_config(key, value) %}
{{ key }} = {{ value }}
{% endmacro %}
```

### 5. 更好的错误处理
- 详细的错误信息
- 准确的行号定位
- 语法验证

## 使用示例

### 基本用法

```rust
use rs_ansible::{AnsibleManager, TemplateOptions, HostConfig};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    
    // 准备模板变量
    let mut variables = HashMap::new();
    variables.insert("app_name".to_string(), "myapp".to_string());
    variables.insert("port".to_string(), "8080".to_string());
    variables.insert("environment".to_string(), "production".to_string());
    
    // 部署模板
    let options = TemplateOptions {
        src: "templates/config.conf.tera".to_string(),
        dest: "/etc/myapp/config.conf".to_string(),
        variables,
        mode: Some("0644".to_string()),
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        backup: true,
        validate: None,
    };
    
    let result = manager.deploy_template_to_hosts(&options, &["web-server".to_string()]).await;
    
    for (host, res) in result.results {
        match res {
            Ok(template_result) => {
                println!("✓ {}: 部署成功 (changed: {})", host, template_result.changed);
            }
            Err(e) => {
                println!("✗ {}: 部署失败: {}", host, e);
            }
        }
    }
    
    Ok(())
}
```

## 测试

运行示例来测试 Tera 功能：

```bash
# 测试 Tera 功能
cargo run --example tera_features

# 测试模板部署（需要配置实际主机）
cargo run --example template_example
```

## 与 User 模块的集成

User 和 Template 模块都已经集成到系统中：

1. **User 模块** (`src/ssh/user.rs`)
   - 用户创建、删除
   - 用户属性修改
   - 组管理
   - 密码管理

2. **Template 模块** (`src/ssh/template.rs`)
   - 使用 Tera 引擎渲染模板
   - 文件备份
   - 权限和所有权管理
   - 配置验证

两个模块可以配合使用：
```rust
// 先创建用户
let user_options = UserOptions {
    name: "myapp".to_string(),
    state: UserState::Present,
    shell: Some("/bin/bash".to_string()),
    home: Some("/home/myapp".to_string()),
    create_home: true,
    // ...
};
manager.manage_user_on_hosts(&user_options, &["web-server".to_string()]).await;

// 然后部署配置文件
let template_options = TemplateOptions {
    src: "config.conf.tera".to_string(),
    dest: "/home/myapp/config.conf".to_string(),
    owner: Some("myapp".to_string()),
    group: Some("myapp".to_string()),
    // ...
};
manager.deploy_template_to_hosts(&template_options, &["web-server".to_string()]).await;
```

## 下一步

可以考虑添加的功能：

1. **自定义 Tera 过滤器**
   - 添加项目特定的过滤器
   - 例如：密码哈希、路径规范化等

2. **模板缓存**
   - 缓存已编译的模板以提高性能

3. **模板验证**
   - 在部署前验证模板语法

4. **模板仓库**
   - 维护一个常用模板库
   - 支持模板版本管理

5. **变量验证**
   - 定义模板所需的变量 schema
   - 在渲染前验证变量完整性
