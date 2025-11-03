# Tera 模板引擎使用指南

本项目使用 [Tera](https://tera.netlify.app/) 作为模板引擎，支持丰富的模板功能。

## 目录

- [基本语法](#基本语法)
- [变量](#变量)
- [过滤器](#过滤器)
- [控制结构](#控制结构)
- [注释](#注释)
- [模板继承](#模板继承)
- [示例模板](#示例模板)

## 基本语法

### 变量

使用双花括号 `{{ }}` 来输出变量：

```jinja2
{{ variable_name }}
{{ user.name }}
{{ config.database.host }}
```

### 语句

使用 `{% %}` 来编写控制语句：

```jinja2
{% if condition %}
    ...
{% endif %}

{% for item in items %}
    ...
{% endfor %}
```

## 变量

### 基本变量

```jinja2
应用名称: {{ app_name }}
端口: {{ port }}
环境: {{ environment }}
```

### 默认值

使用 `default` 过滤器设置默认值：

```jinja2
端口: {{ port | default(value="8080") }}
主机: {{ host | default(value="0.0.0.0") }}
```

### 访问对象属性

```jinja2
用户名: {{ user.name }}
邮箱: {{ user.email }}
城市: {{ user.address.city }}
```

### 访问数组元素

```jinja2
第一个: {{ items.0 }}
第二个: {{ items.1 }}
```

## 过滤器

过滤器用于转换变量的值，使用管道符 `|` 连接：

### 字符串过滤器

```jinja2
大写: {{ name | upper }}
小写: {{ name | lower }}
首字母大写: {{ name | capitalize }}
标题格式: {{ title | title }}
去除空格: {{ text | trim }}
截断: {{ description | truncate(length=100) }}
替换: {{ text | replace(from="old", to="new") }}
```

### 数值过滤器

```jinja2
绝对值: {{ number | abs }}
四舍五入: {{ number | round }}
四舍五入到整数: {{ number | round(method="ceil") }}
向下取整: {{ number | round(method="floor") }}
```

### 数组/对象过滤器

```jinja2
长度: {{ items | length }}
第一个: {{ items | first }}
最后一个: {{ items | last }}
连接: {{ items | join(sep=", ") }}
排序: {{ items | sort }}
反转: {{ items | reverse }}
去重: {{ items | unique }}
```

### 日期过滤器

```jinja2
格式化日期: {{ timestamp | date(format="%Y-%m-%d %H:%M:%S") }}
```

### 自定义过滤器链

可以链式使用多个过滤器：

```jinja2
{{ name | lower | capitalize }}
{{ items | sort | first }}
{{ description | truncate(length=50) | upper }}
```

## 控制结构

### 条件语句

#### if-else

```jinja2
{% if environment == "production" %}
debug = false
log_level = error
{% else %}
debug = true
log_level = debug
{% endif %}
```

#### elif

```jinja2
{% if score >= 90 %}
等级: A
{% elif score >= 80 %}
等级: B
{% elif score >= 70 %}
等级: C
{% else %}
等级: D
{% endif %}
```

#### 逻辑运算符

```jinja2
{% if user.is_admin and user.is_active %}
    管理员权限已激活
{% endif %}

{% if status == "ready" or status == "pending" %}
    系统准备就绪
{% endif %}

{% if not user.is_banned %}
    欢迎回来！
{% endif %}
```

### 循环

#### 基本循环

```jinja2
{% for server in servers %}
upstream_server_{{ loop.index }} = {{ server }}
{% endfor %}
```

#### 循环变量

在循环中可以使用以下特殊变量：

- `loop.index`: 当前迭代索引（从 1 开始）
- `loop.index0`: 当前迭代索引（从 0 开始）
- `loop.first`: 是否第一次迭代
- `loop.last`: 是否最后一次迭代
- `loop.length`: 循环总次数

```jinja2
{% for item in items %}
    {% if loop.first %}
    # 第一项
    {% endif %}
    
    项目 {{ loop.index }}/{{ loop.length }}: {{ item.name }}
    
    {% if loop.last %}
    # 最后一项
    {% endif %}
{% endfor %}
```

#### 空列表处理

```jinja2
{% for user in users %}
    用户: {{ user.name }}
{% else %}
    没有用户
{% endfor %}
```

#### 循环对象

```jinja2
{% for key, value in config %}
{{ key }} = {{ value }}
{% endfor %}
```

### 宏（Macros）

定义可重用的模板片段：

```jinja2
{% macro render_input(name, type="text", placeholder="") %}
<input type="{{ type }}" 
       name="{{ name }}" 
       placeholder="{{ placeholder }}" />
{% endmacro %}

<!-- 使用宏 -->
{{ render_input(name="username", placeholder="请输入用户名") }}
{{ render_input(name="password", type="password") }}
```

## 注释

```jinja2
{# 这是单行注释 #}

{#
这是
多行注释
#}
```

## 模板继承

### 基础模板 (base.conf)

```jinja2
# 基础配置文件
# 项目: {{ project_name }}

{% block header %}
# 默认头部
{% endblock header %}

{% block content %}
# 默认内容
{% endblock content %}

{% block footer %}
# 默认尾部
{% endblock footer %}
```

### 继承模板

```jinja2
{% extends "base.conf" %}

{% block header %}
# 自定义头部
应用名称: {{ app_name }}
版本: {{ version }}
{% endblock header %}

{% block content %}
# 应用配置
port = {{ port }}
host = {{ host }}
{% endblock content %}
```

## 示例模板

### Nginx 配置示例

参见 `examples/nginx.conf.tera`

### 应用配置示例

参见 `examples/app.conf.tera`

### Systemd 服务文件

```jinja2
[Unit]
Description={{ service_description | default(value="Service") }}
After=network.target

[Service]
Type={{ service_type | default(value="simple") }}
User={{ service_user }}
Group={{ service_group | default(value=service_user) }}
WorkingDirectory={{ working_directory }}
ExecStart={{ exec_start }}
{% if exec_reload %}
ExecReload={{ exec_reload }}
{% endif %}
Restart={{ restart_policy | default(value="on-failure") }}
RestartSec={{ restart_sec | default(value="5") }}

{% if environment_vars %}
# 环境变量
{% for key, value in environment_vars %}
Environment="{{ key }}={{ value }}"
{% endfor %}
{% endif %}

[Install]
WantedBy=multi-user.target
```

### Docker Compose 配置

```jinja2
version: '3.8'

services:
  {{ service_name }}:
    image: {{ docker_image }}:{{ docker_tag | default(value="latest") }}
    container_name: {{ container_name | default(value=service_name) }}
    
    {% if ports %}
    ports:
      {% for port in ports %}
      - "{{ port.host }}:{{ port.container }}"
      {% endfor %}
    {% endif %}
    
    {% if volumes %}
    volumes:
      {% for volume in volumes %}
      - {{ volume.host }}:{{ volume.container }}{% if volume.readonly %}:ro{% endif %}
      {% endfor %}
    {% endif %}
    
    {% if environment %}
    environment:
      {% for key, value in environment %}
      {{ key }}: {{ value }}
      {% endfor %}
    {% endif %}
    
    {% if depends_on %}
    depends_on:
      {% for service in depends_on %}
      - {{ service }}
      {% endfor %}
    {% endif %}
    
    restart: {{ restart_policy | default(value="unless-stopped") }}
```

## 在 Rust 代码中使用

```rust
use std::collections::HashMap;
use rs_ansible::{AnsibleManager, TemplateOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = AnsibleManager::new();
    
    // 准备变量
    let mut variables = HashMap::new();
    variables.insert("app_name".to_string(), "myapp".to_string());
    variables.insert("port".to_string(), "8080".to_string());
    variables.insert("environment".to_string(), "production".to_string());
    
    // 配置模板选项
    let options = TemplateOptions {
        src: "templates/app.conf.tera".to_string(),
        dest: "/etc/myapp/config.conf".to_string(),
        variables,
        mode: Some("0644".to_string()),
        owner: Some("myapp".to_string()),
        group: Some("myapp".to_string()),
        backup: true,
        validate: Some("myapp --check-config %s".to_string()),
    };
    
    // 部署模板
    let result = manager.deploy_template_to_all(&options).await;
    
    println!("成功: {}", result.successful.len());
    println!("失败: {}", result.failed.len());
    
    Ok(())
}
```

## 更多资源

- [Tera 官方文档](https://tera.netlify.app/docs/)
- [Tera 内置过滤器列表](https://tera.netlify.app/docs/#built-in-filters)
- [Tera 内置测试列表](https://tera.netlify.app/docs/#built-in-tests)
- [Tera GitHub 仓库](https://github.com/Keats/tera)
