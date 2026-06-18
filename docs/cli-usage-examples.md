# requests-go CLI 跨语言调用示例

requests-go CLI 通过 JSON stdin/stdout 与任何编程语言交互。

## 基本模式

```
输入:  JSON → stdin 或 --json 参数
输出:  JSON → stdout
错误:  文本 → stderr
退出码: 0=成功, 1=失败
```

## Python

```python
import subprocess
import json

def requests_go(method, url, **kwargs):
    """调用 requests-go CLI 发起 HTTP 请求"""
    input_data = {"method": method, "url": url, **kwargs}
    result = subprocess.run(
        ["./requests-go", "request", "--json", json.dumps(input_data)],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        raise Exception(result.stderr)
    return json.loads(result.stdout)

# 使用
resp = requests_go("GET", "https://httpbin.org/get",
                   headers={"User-Agent": "Mozilla/5.0"})
print(resp["status_code"])
```

## Node.js

```javascript
const { spawn } = require('child_process');

function requestsGo(method, url, opts = {}) {
  return new Promise((resolve, reject) => {
    const input = JSON.stringify({ method, url, ...opts });
    const child = spawn('./requests-go', ['request', '--json', input]);
    let stdout = '', stderr = '';
    child.stdout.on('data', d => stdout += d);
    child.stderr.on('data', d => stderr += d);
    child.on('close', code => {
      if (code !== 0) reject(new Error(stderr));
      else resolve(JSON.parse(stdout));
    });
  });
}

// 使用
requestsGo('GET', 'https://httpbin.org/get', {
  headers: { 'User-Agent': 'Mozilla/5.0' }
}).then(console.log);
```

## Ruby

```ruby
require 'json'
require 'open3'

def requests_go(method, url, **opts)
  input = { method: method, url: url }.merge(opts).to_json
  stdout, stderr, status = Open3.capture3(
    './requests-go', 'request', '--json', input
  )
  raise stderr unless status.success?
  JSON.parse(stdout)
end

resp = requests_go('GET', 'https://httpbin.org/get')
puts resp['status_code']
```

## Java

```java
import java.io.*;
import com.google.gson.*;

public class RequestsGo {
    public static JsonObject request(String method, String url,
                                      JsonObject opts) throws Exception {
        JsonObject input = new JsonObject();
        input.addProperty("method", method);
        input.addProperty("url", url);
        if (opts != null) opts.entrySet().forEach(
            e -> input.add(e.getKey(), e.getValue()));

        ProcessBuilder pb = new ProcessBuilder(
            "./requests-go", "request", "--json", input.toString());
        Process p = pb.start();

        String stdout = new String(p.getInputStream().readAllBytes());
        String stderr = new String(p.getErrorStream().readAllBytes());

        if (p.waitFor() != 0) throw new RuntimeException(stderr);
        return JsonParser.parseString(stdout).getAsJsonObject();
    }
}
```

## Rust

```rust
use std::process::Command;
use serde_json::{json, Value};

fn requests_go(method: &str, url: &str, opts: Option<Value>) -> Result<Value, String> {
    let mut input = json!({"method": method, "url": url});
    if let Some(o) = opts {
        input.as_object_mut().unwrap().extend(o.as_object().unwrap().clone());
    }

    let output = Command::new("./requests-go")
        .args(["request", "--json", &input.to_string()])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}
```

## PHP

```php
function requests_go(string $method, string $url, array $opts = []): array {
    $input = json_encode(array_merge(
        ['method' => $method, 'url' => $url], $opts
    ));
    $process = proc_open(
        ['./requests-go', 'request', '--json', $input],
        [1 => ['pipe', 'w'], 2 => ['pipe', 'w']],
        $pipes
    );
    $stdout = stream_get_contents($pipes[1]);
    $stderr = stream_get_contents($pipes[2]);
    $exitCode = proc_close($process);

    if ($exitCode !== 0) throw new Exception($stderr);
    return json_decode($stdout, true);
}
```
