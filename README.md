# RS ANSIBLE

一个使用 Rust 编写的类似 Ansible 的配置管理工具。

## 功能特性

- **SSH 连接管理**: 支持远程主机的 SSH 连接和命令执行
- **模板部署**: 使用 Tera 模板引擎进行配置文件部署
- **用户管理**: 创建、修改和管理远程用户
- **并发执行**: 支持在多个主机上并发执行任务
- **文件传输**: 支持文件上传、下载和哈希校验

## 模板功能

模板功能使用 [Tera](https://tera.netlify.app/) 模板引擎，支持丰富的模板语法：

### 基本用法

```rust
use rs_ansible::{AnsibleManager, TemplateOptions};
use std::collections::HashMap;

let manager = AnsibleManager::new();

// 创建变量映射
let mut variables = HashMap::new();
variables.insert("app_name".to_string(), "myapp".to_string());
variables.insert("port".to_string(), "8080".to_string());
variables.insert("environment".to_string(), "production".to_string());

// 配置模板选项
let options = TemplateOptions {
    src: "/path/to/template.conf".to_string(),
    dest: "/etc/myapp/config.conf".to_string(),
    variables,
    mode: Some("0644".to_string()),
    owner: Some("root".to_string()),
    group: Some("root".to_string()),
    backup: true,
    validate: Some("myapp --check-config %s".to_string()),
};

// 部署模板
let result = manager.deploy_template("host1", &options).await?;
```

### 模板语法示例

#### 变量替换
```jinja2
# 基本变量
app_name = {{ app_name }}
port = {{ port }}

# 带过滤器的变量
name = {{ app_name | upper }}
environment = {{ environment | lower }}
```

#### 条件语句
```jinja2
{% if environment == "production" %}
debug = false
log_level = error
{% else %}
debug = true
log_level = debug
{% endif %}
```

#### 循环
```jinja2
# 假设变量中有一个序列
{% for server in servers %}
upstream_server {{ loop.index }} = {{ server }}
{% endfor %}
```

#### 包含和继承
```jinja2
{% extends "base.conf" %}

{% block content %}
# 自定义内容
server_name = {{ app_name }}
{% endblock %}
```

### Tera 支持的过滤器

- **字符串过滤器**: `upper`, `lower`, `capitalize`, `title`, `trim`, `truncate`
- **数值过滤器**: `round`, `abs`
- **数组过滤器**: `first`, `last`, `join`, `length`
- **日期过滤器**: `date`
- 更多过滤器请参考 [Tera 文档](https://tera.netlify.app/docs/#built-in-filters)

### 模板验证

可以在部署前验证模板渲染后的内容：

```rust
let options = TemplateOptions {
    // ... 其他配置
    validate: Some("nginx -t -c %s".to_string()), // %s 会被替换为临时文件路径
    // ...
};
```

## 用户管理

```rust
use rs_ansible::{AnsibleManager, UserOptions, UserState};

let options = UserOptions {
    name: "deploy".to_string(),
    state: UserState::Present,
    password: Some("hashed_password".to_string()),
    shell: Some("/bin/bash".to_string()),
    home: Some("/home/deploy".to_string()),
    groups: vec!["sudo".to_string(), "docker".to_string()],
    system: false,
    create_home: true,
    uid: None,
    gid: None,
    comment: Some("Deployment user".to_string()),
    expires: None,
};

let result = manager.manage_user("host1", &options).await?;
```

## 许可证

MIT