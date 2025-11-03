# Tera 模板快速参考

## 基本语法

| 功能 | 语法 | 示例 |
|------|------|------|
| 输出变量 | `{{ variable }}` | `{{ app_name }}` |
| 注释 | `{# comment #}` | `{# 这是注释 #}` |
| 语句块 | `{% statement %}` | `{% if condition %}` |

## 变量和过滤器

```jinja2
# 基本变量
{{ name }}

# 默认值
{{ port | default(value="8080") }}

# 过滤器链
{{ name | lower | capitalize }}

# 常用过滤器
{{ text | upper }}          # 大写
{{ text | lower }}          # 小写
{{ text | trim }}           # 去空格
{{ text | truncate(length=50) }}  # 截断
{{ list | length }}         # 长度
{{ list | first }}          # 第一个
{{ list | last }}           # 最后一个
{{ list | join(sep=", ") }} # 连接
```

## 条件语句

```jinja2
{% if condition %}
    ...
{% elif other_condition %}
    ...
{% else %}
    ...
{% endif %}

# 逻辑运算符
{% if a and b %}...{% endif %}
{% if a or b %}...{% endif %}
{% if not a %}...{% endif %}

# 比较运算符
{% if x == y %}...{% endif %}
{% if x != y %}...{% endif %}
{% if x > y %}...{% endif %}
{% if x < y %}...{% endif %}
{% if x >= y %}...{% endif %}
{% if x <= y %}...{% endif %}
```

## 循环

```jinja2
{% for item in items %}
    {{ item }}
{% endfor %}

# 循环变量
{{ loop.index }}    # 从 1 开始的索引
{{ loop.index0 }}   # 从 0 开始的索引
{{ loop.first }}    # 是否第一个
{{ loop.last }}     # 是否最后一个
{{ loop.length }}   # 总数

# 空列表处理
{% for item in items %}
    {{ item }}
{% else %}
    列表为空
{% endfor %}

# 遍历对象
{% for key, value in object %}
    {{ key }}: {{ value }}
{% endfor %}
```

## 宏

```jinja2
# 定义宏
{% macro input(name, type="text") %}
<input type="{{ type }}" name="{{ name }}" />
{% endmacro %}

# 使用宏
{{ input(name="username") }}
{{ input(name="password", type="password") }}
```

## 模板继承

```jinja2
# 父模板 (base.html)
<!DOCTYPE html>
<html>
<head>
    {% block head %}
    <title>{% block title %}{% endblock %}</title>
    {% endblock %}
</head>
<body>
    {% block content %}{% endblock %}
</body>
</html>

# 子模板
{% extends "base.html" %}

{% block title %}我的页面{% endblock %}

{% block content %}
    <h1>内容</h1>
{% endblock %}
```

## 包含

```jinja2
# 包含其他模板
{% include "header.html" %}
```

## 测试

```jinja2
{% if value is defined %}...{% endif %}
{% if value is undefined %}...{% endif %}
{% if list is iterable %}...{% endif %}
{% if value is number %}...{% endif %}
{% if value is string %}...{% endif %}
{% if list is containing(item) %}...{% endif %}
{% if value is starting_with("prefix") %}...{% endif %}
{% if value is ending_with("suffix") %}...{% endif %}
{% if value is matching("regex") %}...{% endif %}
```

## 常用配置模板模式

### Nginx 配置
```jinja2
server {
    listen {{ port | default(value="80") }};
    server_name {{ server_name }};
    root {{ web_root }};
    
    {% if ssl_enabled %}
    listen 443 ssl;
    ssl_certificate {{ ssl_cert }};
    ssl_certificate_key {{ ssl_key }};
    {% endif %}
    
    location / {
        try_files $uri $uri/ =404;
    }
}
```

### 应用配置
```jinja2
[app]
name = {{ app_name }}
port = {{ port }}
{% if environment == "production" %}
debug = false
{% else %}
debug = true
{% endif %}

[database]
host = {{ db_host }}
port = {{ db_port | default(value="5432") }}
```

### Systemd 服务
```jinja2
[Unit]
Description={{ description }}
After=network.target

[Service]
Type=simple
User={{ user }}
ExecStart={{ exec_start }}
Restart={{ restart | default(value="on-failure") }}

{% for key, value in environment %}
Environment="{{ key }}={{ value }}"
{% endfor %}

[Install]
WantedBy=multi-user.target
```

### Docker Compose
```jinja2
version: '3.8'
services:
  {{ service_name }}:
    image: {{ image }}:{{ tag | default(value="latest") }}
    {% if ports %}
    ports:
      {% for port in ports %}
      - "{{ port }}"
      {% endfor %}
    {% endif %}
    {% if volumes %}
    volumes:
      {% for volume in volumes %}
      - {{ volume }}
      {% endfor %}
    {% endif %}
```

## 调试技巧

```jinja2
# 输出变量类型和值
{{ variable }} {# 直接输出 #}
{{ variable | json_encode() }} {# JSON 格式输出 #}

# 检查变量是否定义
{% if variable is defined %}
    已定义: {{ variable }}
{% else %}
    未定义
{% endif %}

# 输出所有上下文变量（调试用）
{{ __tera_context }}
```

## 性能优化

1. **避免在循环中使用复杂过滤器**
```jinja2
# 不好
{% for item in items %}
    {{ item.text | upper | truncate(length=100) | trim }}
{% endfor %}

# 好 - 在 Rust 中预处理
{% for item in processed_items %}
    {{ item }}
{% endfor %}
```

2. **使用宏减少重复**
```jinja2
{% macro render_item(item) %}
    <div>{{ item.name }}</div>
{% endmacro %}

{% for item in items %}
    {{ render_item(item=item) }}
{% endfor %}
```

3. **条件判断放在循环外**
```jinja2
# 不好
{% for item in items %}
    {% if show_details %}
        详细信息: {{ item }}
    {% endif %}
{% endfor %}

# 好
{% if show_details %}
    {% for item in items %}
        详细信息: {{ item }}
    {% endfor %}
{% endif %}
```

## 更多资源

- **官方文档**: https://tera.netlify.app/docs/
- **过滤器列表**: https://tera.netlify.app/docs/#built-in-filters
- **测试列表**: https://tera.netlify.app/docs/#built-in-tests
- **GitHub**: https://github.com/Keats/tera
