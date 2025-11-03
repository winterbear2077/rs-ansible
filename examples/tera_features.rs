use tera::{Tera, Context};

fn main() {
    println!("=== Tera 模板引擎测试 ===\n");
    
    // 测试 1: 基本变量替换
    test_basic_variables();
    
    // 测试 2: 条件语句
    test_conditionals();
    
    // 测试 3: 循环
    test_loops();
    
    // 测试 4: 过滤器
    test_filters();
}

fn test_basic_variables() {
    println!("测试 1: 基本变量替换");
    
    let template = r#"
应用名称: {{ app_name }}
端口: {{ port }}
环境: {{ environment }}
"#;
    
    let mut tera = Tera::default();
    tera.add_raw_template("test", template).unwrap();
    
    let mut context = Context::new();
    context.insert("app_name", "myapp");
    context.insert("port", &8080);
    context.insert("environment", "production");
    
    let result = tera.render("test", &context).unwrap();
    println!("{}", result);
}

fn test_conditionals() {
    println!("测试 2: 条件语句");
    
    let template = r#"
环境: {{ environment }}
{% if environment == "production" %}
日志级别: error
调试模式: 关闭
{% else %}
日志级别: debug
调试模式: 开启
{% endif %}
"#;
    
    let mut tera = Tera::default();
    tera.add_raw_template("test", template).unwrap();
    
    let mut context = Context::new();
    context.insert("environment", "production");
    
    let result = tera.render("test", &context).unwrap();
    println!("{}", result);
}

fn test_loops() {
    println!("测试 3: 循环");
    
    let template = r#"
上游服务器配置:
{% for server in servers %}
  - 服务器 {{ loop.index }}: {{ server.host }}:{{ server.port }}
{% endfor %}
"#;
    
    let mut tera = Tera::default();
    tera.add_raw_template("test", template).unwrap();
    
    let mut context = Context::new();
    
    // 创建服务器列表
    let servers = vec![
        serde_json::json!({"host": "192.168.1.10", "port": 8080}),
        serde_json::json!({"host": "192.168.1.11", "port": 8080}),
        serde_json::json!({"host": "192.168.1.12", "port": 8080}),
    ];
    context.insert("servers", &servers);
    
    let result = tera.render("test", &context).unwrap();
    println!("{}", result);
}

fn test_filters() {
    println!("测试 4: 过滤器");
    
    let template = r#"
原始名称: {{ name }}
大写: {{ name | upper }}
小写: {{ name | lower }}
首字母大写: {{ name | capitalize }}
标题格式: {{ name | title }}
截断(10字符): {{ description | truncate(length=10) }}
"#;
    
    let mut tera = Tera::default();
    tera.add_raw_template("test", template).unwrap();
    
    let mut context = Context::new();
    context.insert("name", "myApplication");
    context.insert("description", "This is a very long description that should be truncated");
    
    let result = tera.render("test", &context).unwrap();
    println!("{}", result);
}
