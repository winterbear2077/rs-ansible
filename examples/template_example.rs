use rs_ansible::{AnsibleManager, TemplateOptions};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化管理器
    let mut manager = AnsibleManager::new();
    
    // 添加主机
    let hosts = [
        "179.10.18.1",
        "179.10.18.2",
        "179.10.18.3",
        "179.10.18.4",
        "179.10.18.5",
        "179.10.18.6",
        "179.10.18.7",
        "179.10.18.8",
        "179.10.18.9",
        "179.10.18.10",
    ];

    let _ = hosts.iter().for_each(|&host| {
        manager.add_host(format!("test-server-{}", host),
            AnsibleManager::host_builder()
            .hostname(host)
            .username("root")
            .password("mod.root.0815")
            .port(22)
            .build()
        );
    });

    
    // 示例 1: 部署 Nginx 配置
    // deploy_nginx_config(&manager).await?;
    
    // 示例 2: 部署应用配置
    deploy_app_config(&manager).await?;
    
    Ok(())
}

async fn deploy_nginx_config(manager: &AnsibleManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("部署 Nginx 配置...");
    
    let mut variables:HashMap<String, serde_json::Value> = HashMap::new();
    variables.insert("app_name".to_string(), "myapp".to_string().into());
    variables.insert("server_name".to_string(), "example.com".to_string().into());
    variables.insert("port".to_string(), "80".to_string().into());
    variables.insert("environment".to_string(), "production".to_string().into());
    variables.insert("web_root".to_string(), "/var/www/myapp".to_string().into());
    variables.insert("ssl_enabled".to_string(), "false".to_string().into());
    variables.insert("enable_cache".to_string(), "true".to_string().into());
    
    let options = TemplateOptions {
        src: "examples/nginx.conf.tera".to_string(),
        dest: "/etc/nginx/sites-available/myapp.conf".to_string(),
        variables,
        mode: Some("0644".to_string()),
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        backup: true,
        validate: Some("nginx -t -c %s".to_string()),
    };
    

    let mut hosts = vec![];
    for i in 1..=10 {
        hosts.push(format!("test-server-{i}"));
    }
    let batch_result = manager.deploy_template_to_hosts(&options, &hosts).await;
    
    // 检查结果
    if let Some(result) = batch_result.results.get("test-server-1") {
        match result {
            Ok(template_result) => {
                println!("✓ Nginx 配置部署成功");
                if template_result.changed {
                    println!("  配置已更新");
                    if let Some(ref diff) = template_result.diff {
                        println!("  差异:\n{}", diff);
                    }
                } else {
                    println!("  配置未变更");
                }
            }
            Err(e) => {
                println!("✗ 部署失败: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn deploy_app_config(manager: &AnsibleManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n部署应用配置...");
    
    let mut variables: HashMap<String, serde_json::Value> = HashMap::new();
    variables.insert("app_name".to_string(), "myapp".to_string().into());
    variables.insert("version".to_string(), "2.1.0".to_string().into());
    variables.insert("environment".to_string(), "production".to_string().into());
    variables.insert("host".to_string(), "0.0.0.0".to_string().into());
    variables.insert("port".to_string(), "8080".to_string().into());
    variables.insert("workers".to_string(), "8".to_string().into());
    variables.insert("db_host".to_string(), "db.example.com".to_string().into());
    variables.insert("db_port".to_string(), "5432".to_string().into());
    variables.insert("db_name".to_string(), "myapp_db".to_string().into());
    variables.insert("db_user".to_string(), "myapp_user".to_string().into());
    variables.insert("enable_redis".to_string(), "true".to_string().into());
    variables.insert("redis_host".to_string(), "cache.example.com".to_string().into());
    
    // 使用 Tera 的内置过滤器获取当前时间
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    variables.insert("generation_time".to_string(), now.into());
    
    let options = TemplateOptions {
        src: "examples/app.conf.tera".to_string(),
        dest: "/etc/myapp/config.ini".to_string(),
        variables,
        mode: Some("0640".to_string()),
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        backup: true,
        validate: None, // 可以添加配置验证命令
    };

    let hosts = [
        "179.10.18.1",
        "179.10.18.2",
        "179.10.18.3",
        "179.10.18.4",
        "179.10.18.5",
        "179.10.18.6",
        "179.10.18.7",
        "179.10.18.8",
        "179.10.18.9",
        "179.10.18.10",
    ];
    let hostnames: Vec<String> = hosts.iter().map(|&host| format!("test-server-{host}")).collect();

    // println!("{:?}", manager.list_hosts());
    
    let batch_result = manager.deploy_template_to_hosts(&options, &hostnames).await;
    

    // println!("{:?}", batch_result);
    // 检查结果
    if let Some(result) = batch_result.results.get("test-server-179.10.18.1") {
        match result {
            Ok(template_result) => {
                println!("✓ 应用配置部署成功");
                if template_result.changed {
                    println!("  配置已更新");
                } else {
                    println!("  配置未变更");
                }
            }
            Err(e) => {
                println!("✗ 部署失败: {}", e);
            }
        }
    }
    
    Ok(())
}
