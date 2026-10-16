# requests

[![Rust Version](https://img.shields.io/badge/Rust-1.70%2B-blue?logo=rust&logoColor=white&labelColor=gray)](https://www.rust-lang.org)[![License](https://img.shields.io/badge/license-GPL3.0-orange)](LICENSE)

基于 Rust + reqwest 的 HTTP 客户端库，专注于**反爬虫**与**浏览器指纹模拟**能力。核心库可编译为 `.so` / `.dylib` / `.dll`，并通过 FFI 供 Python、Node.js、Java、PHP、Ruby 等语言调用；同时提供开箱即用的 CLI 和多语言上层封装。

核心特性：
- 🔐 **TLS/JA3 指纹**：支持自定义 JA3 字符串与随机 JA3
- 🌐 **HTTP/2 指纹**：可配置 HTTP/2 settings、伪头顺序、优先级帧
- 🧭 **curl-impersonate 浏览器配置集**：`Chrome110/120`、`Firefox115`、`Safari15`、`Edge110`
- 🔁 **重试与超时**：内置重试策略、连接超时、跳转控制
- 🍪 **Cookie 持久化**：支持 cookie 文件读写
- ⚙️ **连接池调优**：可配置 max idle、idle timeout、TCP keepalive
- 📦 **多语言可调用**：FFI + 多语言 wrapper，像用本地 SDK 一样发请求
- 📁 **多部分上传**：支持 multipart/form-data 文件上传
- 🔑 **认证与会话默认值**：Basic Auth、Bearer Token、默认 UA/头、本地地址绑定、TCP 参数
- 🔄 **UA 池与重试退避**：每次重试可轮换 User-Agent，支持指数退避
- 🌍 **DNS 控制**：支持 DNS 缓存、自定义 DNS 服务器
- 🔗 **HTTP/2 精细控制**：支持 HTTP/2 伪头顺序、连接复用开关
- 🔒 **TLS 版本/ALPN**：可限制 TLS 版本范围、自定义 ALPN 列表

---

## 目录

- [快速开始](#快速开始)
- [Rust 库使用](#rust-库使用)
- [CLI 使用](#cli-使用)
- [构建共享库](#构建共享库)
- [多语言 Wrapper](#多语言-wrapper)
- [反爬与指纹配置详解](#反爬与指纹配置详解)
- [中间件](#中间件)
- [常见问题](#常见问题)

---

## 快速开始

### 1) 作为 Rust 依赖使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
requests-core = { path = "rust/requests-core" }
```

基本示例：

```rust
use requests_core::{Request, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new();
    let resp = session.get("https://httpbin.org/get").await?;
    println!("status: {}", resp.status_code.as_u16());
    println!("body: {}", resp.text());
    Ok(())
}
```

### 2) 作为 CLI / 共享库调用

项目可直接编译为二进制与共享库：

```bash
# 编译 CLI
cargo build --release -p requests-cli

# 编译当前平台的 .so / .dylib / .dll
bash libs_dist/build-all.sh
```

产物位于：

```text
libs_dist/
├── requests-linux-amd64.so
├── requests-linux-arm64.so
├── requests-darwin-amd64.dylib
├── requests-darwin-arm64.dylib
└── requests-windows-amd64.dll
```

---

## Rust 库使用

### 会话与会话复用

```rust
use requests_core::{Request, RequestParams, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut session = Session::new()
        .with_timeout(std::time::Duration::from_secs(30))
        .with_retry(3)
        .with_retry_delay(std::time::Duration::from_secs(1))
        .with_allow_redirects(true);

    // 多个请求复用同一会话（cookie jar、代理、TLS 配置等）
    let r1 = session.get("https://httpbin.org/get").await?;
    let r2 = session.get("https://httpbin.org/headers").await?;
    Ok(())
}
```

### 自定义请求头

```rust
use requests_core::{Request, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new();
    let mut req = Request::default();
    req.headers.insert("User-Agent", "Mozilla/5.0 ...");
    req.headers.insert("Accept", "application/json");

    let resp = session.get("https://httpbin.org/get", req).await?;
    println!("{}", resp.text());
    Ok(())
}
```

### GET 参数

```rust
use requests_core::{Request, RequestParams, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new();
    let mut req = Request::default();
    let mut params = RequestParams::new();
    params.add("page", "1");
    params.add("size", "20");
    req.params = Some(params);

    let resp = session.get("https://httpbin.org/get", req).await?;
    println!("{}", resp.text());
    Ok(())
}
```

### POST 表单与 JSON

```rust
use requests_core::{Request, RequestParams, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new();

    // 表单
    let mut form = RequestParams::new();
    form.add("username", "admin");
    form.add("password", "123456");
    let resp = session.post("https://httpbin.org/post", Request {
        data: Some(form),
        ..Default::default()
    }).await?;

    // JSON
    let resp = session.post("https://httpbin.org/post", Request {
        json: Some(serde_json::json!({"hello":"world"})),
        ..Default::default()
    }).await?;

    // 原始 Body
    let resp = session.post("https://httpbin.org/post", Request {
        body: Some(b"raw bytes".to_vec()),
        ..Default::default()
    }).await?;

    Ok(())
}
```

### 流式响应

```rust
use requests_core::{Request, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut req = Request::default();
    req.stream = true;
    let resp = Session::new()
        .get("https://httpbin.org/stream-bytes/10240", req)
        .await?;
    println!("status: {}", resp.status_code.as_u16());
    println!("bytes: {}", resp.content.len());
    Ok(())
}
```

### 重定向控制

```rust
use requests_core::Session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new()
        .with_max_redirects(0) // 或请求级 allow_redirects = false
        .with_allow_redirects(false);

    let resp = session.get("http://github.com").await?;
    println!("status: {}", resp.status_code.as_u16());
    Ok(())
}
```

### 代理

```rust
use requests_core::Session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new()
        .with_proxy("http://127.0.0.1:7890")
        .with_verify(false);

    let resp = session.get("https://httpbin.org/ip").await?;
    println!("{}", resp.text());
    Ok(())
}
```

### 客户端证书

```rust
use requests_core::{Request, Session};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new().with_cert(vec![
        "client.crt".into(),
        "client.key".into(),
    ]);
    let resp = session.get("https://example.com").await?;
    Ok(())
}
```

### Cookie 持久化

```rust
use requests_core::Session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new()
        .with_cookie_store_path("cookies.json");

    // 发请求后保存 cookies
    session.get("https://httpbin.org/cookies/set?foo=bar").await?;
    session.save_cookies(&[])?;

    // 下次启动时加载 cookies
    if let Some(cookies) = session.load_cookies()? {
        println!("loaded cookies: {:?}", cookies);
    }
    Ok(())
}
```

### 连接池调优

```rust
use requests_core::Session;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let session = Session::new()
        .with_pool_max_idle_per_host(16)
        .with_pool_idle_timeout(std::time::Duration::from_secs(30))
        .with_tcp_keepalive(std::time::Duration::from_secs(15));

    let resp = session.get("https://httpbin.org/get").await?;
    Ok(())
}
```

---

## CLI 使用

```bash
# 基础请求
./requests request --url https://httpbin.org/get --method GET

# 带会话选项
./requests request \
  --url https://httpbin.org/post \
  --method POST \
  --header "Content-Type: application/json" \
  --body '{"hello":"world"}' \
  --proxy http://127.0.0.1:7890 \
  --timeout 30 \
  --ja3 "771,..." \
  --browser-profile Chrome110

# 流式输出
./requests stream --url https://httpbin.org/stream-bytes/10240 --chunk-size 4096
```

---

## 构建共享库

```bash
bash libs_dist/build-all.sh
```

产物与说明：

```text
libs_dist/requests-linux-amd64.so        # Linux x86_64
libs_dist/requests-linux-arm64.so        # Linux arm64
libs_dist/requests-darwin-amd64.dylib    # macOS x86_64
libs_dist/requests-darwin-arm64.dylib    # macOS arm64
libs_dist/requests-windows-amd64.dll     # Windows x86_64
```

共享库对外接口为 C ABI：

```c
// 输入/输出均为 JSON 字符串
const char *requests_request(const char *config_json);
void requests_free_response(const char *resp_json);
```

---

## 多语言 Wrapper

各语言封装已拆分到独立仓库，按语言访问对应项目：

- Python：`requests-python`
- Node.js：`requests-nodejs`
- Go：`requests-go`
- Java：`requests-java`
- PHP：`requests-php`
- Ruby：`requests-ruby`

示例（Python）：

```python
from requests_core import version, request

print(version())
resp = request({
    "url": "https://httpbin.org/get",
    "headers": {"User-Agent": "Mozilla/5.0"},
    "ja3": "771,...",
    "timeout": 30,
})
print(resp["status_code"])
```

示例（Node.js）：

```js
const { version, request } = require('requests-core');

(async () => {
  console.log(await version());
  const resp = await request({
    url: 'https://httpbin.org/post',
    json: { hello: 'world' },
    proxy: 'http://127.0.0.1:7890',
  });
  console.log(resp.status_code);
})();
```

---

## 反爬与指纹配置详解

### JA3

```rust
use requests_core::Session;

let session = Session::new()
    .with_ja3("771,4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53,...")
    .with_random_ja3(true);
```

### TLS Extensions

```rust
use requests_core::{Request, Session, TLSExtensions};

let mut req = Request::default();
req.tls_extensions = Some(TLSExtensions {
    supported_signature_algorithms: Some(vec![
        "ECDSAWithP256AndSHA256".into(),
        "ECDSAWithP384AndSHA384".into(),
    ]),
    supported_versions: Some(vec!["1.3".into(), "1.2".into()]),
    key_share_curves: Some(vec!["X25519".into(), "P256".into()]),
    ..Default::default()
});
```

### HTTP/2 Settings

```rust
use requests_core::Session;

let session = Session::new()
    .with_http2_settings(r#"{"INITIAL_WINDOW_SIZE":131072,"MAX_FRAME_SIZE":16384}"#);
```

### curl-impersonate 浏览器配置集

```rust
use requests_core::{Session, BrowserProfile};

let session = Session::new()
    .with_curl_impersonate("/usr/bin/curl-impersonate", BrowserProfile::Chrome120);
```

可用配置集：
- `Chrome110`
- `Chrome120`
- `Firefox115`
- `Safari15`
- `Edge110`

### UA 池与重试退避

```rust
use requests_core::Session;

let session = Session::new()
    .with_retry(5)
    .with_retry_backoff(true)
    .with_retry_delay(std::time::Duration::from_secs(1))
    .with_user_agent_pool(vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ...".into(),
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ...".into(),
    ]);
```

### DNS 控制

```rust
use requests_core::Session;

let session = Session::new()
    .with_dns_cache(true)
    .with_dns_servers(vec![
        "8.8.8.8".into(),
        "1.1.1.1".into(),
    ]);
```

### HTTP/2 伪头顺序

```rust
use requests_core::Session;

let session = Session::new()
    .with_http2_header_order(vec![
        ":authority".into(),
        ":method".into(),
        ":path".into(),
        ":scheme".into(),
        "content-type".into(),
    ]);
```

### TLS 版本与 ALPN

```rust
use requests_core::Session;

let session = Session::new()
    .with_tls_min_version("1.2")
    .with_tls_max_version("1.3")
    .with_tls_alpn(vec!["h2".into(), "http/1.1".into()]);
```

---

## 中间件

Session 支持洋葱模型中间件，适合统一加头、日志、重试等：

```rust
use requests_core::Session;

fn logging_middleware(next: requests_core::MiddlewareFn) -> requests_core::MiddlewareFn {
    return Arc::new(move |ctx| {
        println!("{} {}", ctx.request.method, ctx.request.url);
        Box::pin(async move { next(ctx).await })
    });
}

fn main() -> anyhow::Result<()> {
    let mut session = Session::new();
    session.use_middleware(logging_middleware);
    // ...
    Ok(())
}
```

---

## 常见问题

**Q：为什么建议同时提供 `browser_profile` 和 `ja3/http2_settings`？**  
A：`browser_profile` 走 `curl-impersonate`，指纹最接近真实浏览器；`ja3/http2_settings` 走 reqwest + rustls，适合不需要完整浏览器行为的场景。

**Q：共享库如何释放返回的 JSON 字符串？**  
A：CLI/FFI 层提供的 `requests_free_response` 负责释放；语言 wrapper 一般不需要手动释放。

**Q：cookies 如何持久化？**  
A：`Session` 内部持有 `Arc<reqwest::cookie::Jar>`，同一 `Session` 内请求自动共享 cookies；如需跨进程持久化，可在中间件或 wrapper 中序列化。

**Q：为什么有些平台构建后只有 `.so`，其他平台被 skip？**  
A：`build-all.sh` 仅构建与当前 `rustc` host 匹配的目标；跨平台编译需要额外配置 target。

---

本项目由 [深度求索](https://github.com/deepseek) 相关团队维护。如有问题欢迎提交 Issue / PR。
