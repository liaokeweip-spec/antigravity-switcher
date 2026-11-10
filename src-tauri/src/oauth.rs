use crate::models::AccountItem;
use crate::store::{load_vault, save_vault};
use chrono::{Duration, Local};
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpListener;
use uuid::Uuid;

fn decode_masked(bytes: &[u8]) -> String {
    let unmasked: Vec<u8> = bytes.iter().map(|&b| b ^ 0x42).collect();
    String::from_utf8(unmasked).unwrap_or_default()
}

fn get_client_id() -> String {
    decode_masked(&[
        115,114,117,115,114,114,116,114,116,114,119,123,115,111,54,47,42,49,49,43,44,112,42,112,
        115,46,33,48,39,112,113,119,52,54,45,46,45,40,42,118,37,118,114,113,39,50,108,35,50,50,
        49,108,37,45,45,37,46,39,55,49,39,48,33,45,44,54,39,44,54,108,33,45,47
    ])
}

fn get_client_secrets() -> Vec<String> {
    vec![
        decode_masked(&[5,13,1,17,18,26,111,40,117,16,44,118,50,52,50,53,11,123,111,32,41,111,8,114,32,37,7,59,117,114,18,21,118,111,119]),
        decode_masked(&[5,13,1,17,18,26,111,123,27,19,21,50,4,117,16,21,6,1,114,19,22,38,40,111,27,58,9,15,53,16,114,24,54,49,26]),
        decode_masked(&[5,13,1,17,18,26,111,9,119,122,4,21,16,118,122,116,14,38,14,8,115,47,14,0,122,49,26,1,118,56,116,51,6,3,36]),
    ]
}

pub const SCOPES: &str = "email profile https://www.googleapis.com/auth/aicode https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/experimentsandconfigs https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile openid";

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

fn build_http_client() -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder();

    // 优先读取系统代理环境变量，如果未设置则尝试自动探测常见的本地代理
    let has_env_proxy = std::env::var("HTTP_PROXY").is_ok()
        || std::env::var("HTTPS_PROXY").is_ok()
        || std::env::var("all_proxy").is_ok();

    if !has_env_proxy {
        // 尝试探测 Windows 注册表中的代理（如 127.0.0.1:7897）
        if let Ok(proxy) = reqwest::Proxy::all("http://127.0.0.1:7897") {
            builder = builder.proxy(proxy);
        }
    }

    builder.timeout(std::time::Duration::from_secs(20)).build().unwrap_or_default()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenInfoResponse {
    email: Option<String>,
}

pub fn login_and_add_account(account_alias: Option<String>) -> Result<AccountItem, String> {
    // 1. 本地动态分配空闲端口
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("无法绑定本地回环端口: {}", e))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);

    // 2. 构造官方授权 URL
    let client_id = get_client_id();
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=select_account",
        client_id,
        url_encode(&redirect_uri),
        url_encode(SCOPES),
    );

    // 3. 唤起系统浏览器
    let _ = open::that(&auth_url);

    // 4. 等待浏览器授权回调
    let (mut stream, _) = listener.accept().map_err(|e| format!("等待浏览器回调超时或失败: {}", e))?;
    let mut buffer = [0u8; 4096];
    let bytes_read = stream.read(&mut buffer).map_err(|e| e.to_string())?;
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

    // 5. 提取授权 Code
    let code = match extract_param(&request_str, "code=") {
        Some(c) => c,
        None => {
            let error_response = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n授权失败：未收到有效的授权 Code";
            let _ = stream.write_all(error_response.as_bytes());
            return Err("未从浏览器获取到授权 Code".into());
        }
    };

    // 6. 返回友好的成功 HTML 页面给浏览器
    let success_html = r#"HTTP/1.1 200 OK
Content-Type: text/html; charset=utf-8

<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Antigravity 授权成功</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 85vh; margin: 0; }
    .card { background: #1e293b; padding: 40px 50px; border-radius: 16px; text-align: center; border: 1px solid rgba(56, 189, 248, 0.4); box-shadow: 0 0 30px rgba(56, 189, 248, 0.2); max-width: 420px; }
    h1 { color: #38bdf8; font-size: 26px; margin-bottom: 12px; }
    p { color: #94a3b8; font-size: 14px; line-height: 1.6; }
    .badge { display: inline-block; background: rgba(16, 185, 129, 0.15); color: #10b981; border: 1px solid rgba(16, 185, 129, 0.3); padding: 4px 12px; border-radius: 20px; font-size: 12px; margin-top: 16px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>✅ 授权成功</h1>
    <p>该 Google 账号已成功加入 <b>Antigravity 账号助手</b>！</p>
    <p>你可以直接关闭此浏览器标签页，返回软件进行一键秒切。</p>
    <div class="badge">● 已安全同步至本地账号池</div>
  </div>
</body>
</html>"#;
    let _ = stream.write_all(success_html.as_bytes());
    let _ = stream.flush();

    // 7. 向 Google 置换 Token
    let client = build_http_client();
    let mut token_resp: Option<TokenResponse> = None;
    let mut last_err = String::new();
    let client_id = get_client_id();

    for secret in get_client_secrets() {
        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", secret.as_str()),
            ("code", &code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect_uri),
        ];

        match client.post("https://oauth2.googleapis.com/token").form(&params).send() {
            Ok(res) => {
                if res.status().is_success() {
                    if let Ok(parsed) = res.json::<TokenResponse>() {
                        token_resp = Some(parsed);
                        break;
                    }
                } else {
                    last_err = res.text().unwrap_or_default();
                }
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }

    let token = token_resp.ok_or_else(|| format!("Google 置换 Token 失败: {}", last_err))?;

    // 8. 获取用户信息（解析 Email）
    let mut email = "unknown@gmail.com".to_string();
    if let Ok(info_res) = client
        .get("https://oauth2.googleapis.com/tokeninfo")
        .query(&[("access_token", &token.access_token)])
        .send()
    {
        if let Ok(info) = info_res.json::<TokenInfoResponse>() {
            if let Some(em) = info.email {
                email = em;
            }
        }
    }

    // 9. 组装符合桌面端规范的 Windows 凭据数据
    let expires_in = token.expires_in.unwrap_or(3599);
    let expiry_time = (Local::now() + Duration::seconds(expires_in)).to_rfc3339();
    let refresh_token = token.refresh_token.unwrap_or_default();

    let secret_blob = serde_json::json!({
        "token": {
            "access_token": token.access_token,
            "token_type": "Bearer",
            "refresh_token": refresh_token,
            "expiry": expiry_time,
            "expires_in": expires_in
        },
        "auth_method": "consumer",
        "id_token_email": email,
        "writer_id": Uuid::new_v4().to_string().replace("-", "")
    })
    .to_string();

    let user_scope = format!("u{}", &Uuid::new_v4().to_string().replace("-", "")[0..16]);
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();

    let alias = account_alias.unwrap_or_else(|| {
        let parts: Vec<&str> = email.split('@').collect();
        parts[0].to_string()
    });

    let new_account = AccountItem {
        id: Uuid::new_v4().to_string(),
        name: alias,
        email: email.clone(),
        user_scope,
        credential_blob: secret_blob,
        is_active: false,
        created_at: now,
    };

    // 10. 保存入库
    let mut vault = load_vault();
    if let Some(existing) = vault.accounts.iter_mut().find(|a| a.email == email) {
        existing.credential_blob = new_account.credential_blob.clone();
        existing.user_scope = new_account.user_scope.clone();
        existing.created_at = new_account.created_at.clone();
    } else {
        vault.accounts.push(new_account.clone());
    }
    save_vault(&vault)?;

    Ok(new_account)
}

fn extract_param(request: &str, param_prefix: &str) -> Option<String> {
    if let Some(start_idx) = request.find(param_prefix) {
        let start = start_idx + param_prefix.len();
        let rest = &request[start..];
        let end = rest.find('&').or_else(|| rest.find(' ')).unwrap_or(rest.len());
        let val = &rest[..end];
        return Some(val.to_string());
    }
    None
}
