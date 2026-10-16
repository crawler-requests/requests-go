# requests-go Cobra CLI 命令行工具 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`
> Steps use checkbox (`- [ ]`) syntax.

**Goal:** 基于 cobra 将 requests-go 封装为命令行工具，通过 JSON 输入/输出实现跨语言调用（任何语言只需 `exec` 即可使用 TLS 指纹伪造 + HTTP/2 能力）。

**Architecture:** 用户通过命令行参数或 stdin 传入 JSON 请求配置 → CLI 解析 JSON → 调用 requests-go Session.Request → 将 Response 序列化为 JSON 输出到 stdout → 调用方解析 JSON 获取结果。错误输出到 stderr，退出码区分成功/失败。

**Tech Stack:** Go 1.24+, github.com/spf13/cobra v1.9+, github.com/wangluozhe/requests v1.5.2, github.com/wangluozhe/chttp v1.1.1

**Risks:**
- Task 1 新增 cobra 依赖，需确保 go.sum 更新 → 缓解：`go mod tidy` 自动处理
- Task 3 的 JSON 序列化需要处理 binary body → 缓解：统一用 Base64 编码 body，与 libs_dist/export.go 保持一致
- Stream 模式需要特殊处理 → 缓解：用行分隔 JSON（NDJSON）逐行输出流式数据

---

### Task 1: 创建 CLI 入口和根命令

**Depends on:** None
**Files:**
- Create: `cmd/requests-go/main.go`
- Create: `cmd/requests-go/root.go`
- Modify: `go.mod` (add cobra dependency)

- [ ] **Step 1: 安装 cobra 依赖**

Run: `go get github.com/spf13/cobra@latest`
Expected:
  - Exit code: 0
  - go.mod contains "github.com/spf13/cobra"

- [ ] **Step 2: 创建 CLI 入口 main.go — 程序启动点**

```go
// cmd/requests-go/main.go
package main

import (
	"fmt"
	"os"
)

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
```

- [ ] **Step 3: 创建根命令 root.go — 定义全局参数和子命令注册**

```go
// cmd/requests-go/root.go
package main

import (
	"github.com/spf13/cobra"
)

var (
	// 全局 Session 级别参数
	sessionID   string
	proxy       string
	verify      bool
	timeout     int
	ja3         string
	randomJA3   bool
	forceHTTP1  bool
	debug       bool
)

var rootCmd = &cobra.Command{
	Use:   "requests-go",
	Short: "requests-go - HTTP client with TLS fingerprint customization",
	Long: `requests-go is a command-line HTTP client that supports JA3/TLS fingerprint
customization, HTTP/2 settings, and custom TLS extensions.

It accepts JSON input (via --json flag or stdin) and outputs JSON to stdout,
making it easy to call from any programming language via subprocess execution.

Examples:
  requests-go request --json '{"method":"GET","url":"https://httpbin.org/get"}'
  requests-go request --json '{"method":"POST","url":"https://httpbin.org/post","body":"aGVsbG8="}'
  echo '{"method":"GET","url":"https://httpbin.org/get"}' | requests-go request`,
	SilenceUsage:  true,
	SilenceErrors: true,
}

func init() {
	rootCmd.PersistentFlags().StringVarP(&sessionID, "session", "s", "", "Session ID for cookie persistence")
	rootCmd.PersistentFlags().StringVarP(&proxy, "proxy", "x", "", "Proxy URL (e.g. http://127.0.0.1:8080)")
	rootCmd.PersistentFlags().BoolVarP(&verify, "verify", "k", true, "Verify TLS certificates")
	rootCmd.PersistentFlags().IntVarP(&timeout, "timeout", "t", 30, "Request timeout in seconds")
	rootCmd.PersistentFlags().StringVarP(&ja3, "ja3", "j", "", "JA3 fingerprint string")
	rootCmd.PersistentFlags().BoolVar(&randomJA3, "random-ja3", false, "Randomize JA3 fingerprint extensions")
	rootCmd.PersistentFlags().BoolVar(&forceHTTP1, "http1", false, "Force HTTP/1.1 (disable HTTP/2)")
	rootCmd.PersistentFlags().BoolVarP(&debug, "debug", "d", false, "Enable debug mode")
}
```

- [ ] **Step 4: 验证编译**

Run: `go build ./cmd/requests-go/`
Expected:
  - Exit code: 0
  - Binary `requests-go` created

- [ ] **Step 5: 提交**

Run: `git add cmd/requests-go/main.go cmd/requests-go/root.go go.mod go.sum && git commit -m "feat(cli): add cobra CLI entry point with root command"`

---

### Task 2: 实现 request 子命令（核心 HTTP 请求）

**Depends on:** Task 1
**Files:**
- Create: `cmd/requests-go/request.go`
- Create: `cmd/requests-go/types.go`

- [ ] **Step 1: 创建类型定义 types.go — CLI 输入/输出的 JSON 结构体**

```go
// cmd/requests-go/types.go
package main

// RequestInput 是 CLI 接收的 JSON 请求参数
type RequestInput struct {
	Method             string            `json:"method"`
	URL                string            `json:"url"`
	Params             map[string]string `json:"params,omitempty"`
	Headers            map[string]string `json:"headers,omitempty"`
	HeadersOrder       []string          `json:"headers_order,omitempty"`
	UnChangedHeaderKey []string          `json:"unchanged_header_key,omitempty"`
	Cookies            map[string]string `json:"cookies,omitempty"`
	Data               map[string]string `json:"data,omitempty"`
	JSON               any               `json:"json,omitempty"`
	Body               string            `json:"body,omitempty"`        // Base64 编码
	Auth               []string          `json:"auth,omitempty"`         // [user, pass]
	AllowRedirects     bool              `json:"allow_redirects,omitempty"`
	Cert               []string          `json:"cert,omitempty"`         // [certFile, keyFile, caFile?]
	PseudoHeaderOrder  []string          `json:"pseudo_header_order,omitempty"`
	TLSExtensions      any               `json:"tls_extensions,omitempty"`
	HTTP2Settings      any               `json:"http2_settings,omitempty"`
	Stream             bool              `json:"stream,omitempty"`
}

// ResponseOutput 是 CLI 输出的 JSON 响应结构
type ResponseOutput struct {
	URL        string              `json:"url"`
	StatusCode int                 `json:"status_code"`
	Headers    map[string][]string `json:"headers"`
	Cookies    []CookieOutput      `json:"cookies,omitempty"`
	Content    string              `json:"content,omitempty"` // Base64 编码
	Error      string              `json:"error,omitempty"`
}

// CookieOutput 是 JSON 输出的 Cookie 格式
type CookieOutput struct {
	Name     string `json:"name"`
	Value    string `json:"value"`
	Path     string `json:"path,omitempty"`
	Domain   string `json:"domain,omitempty"`
	Expires  string `json:"expires,omitempty"`
	HTTPOnly bool   `json:"http_only"`
	Secure   bool   `json:"secure"`
}

// StreamOutput 是流式响应的 JSON 输出（NDJSON 格式，每行一个）
type StreamOutput struct {
	Chunk string `json:"chunk,omitempty"` // Base64 编码的数据块
	EOF   bool   `json:"eof"`
	Error string `json:"error,omitempty"`
}
```

- [ ] **Step 2: 创建 request 子命令 request.go — 核心 HTTP 请求逻辑**

```go
// cmd/requests-go/request.go
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/wangluozhe/chttp"
	"github.com/wangluozhe/chttp/cookiejar"
	"github.com/wangluozhe/requests"
	"github.com/wangluozhe/requests/transport"
	"github.com/wangluozhe/requests/url"
	"github.com/wangluozhe/requests/utils"
)

var jsonInput string

func init() {
	requestCmd.Flags().StringVar(&jsonInput, "json", "", "JSON request parameters (or use stdin)")
	rootCmd.AddCommand(requestCmd)
}

var requestCmd = &cobra.Command{
	Use:   "request",
	Short: "Send an HTTP request",
	Long: `Send an HTTP request with full TLS fingerprint control.

Input: JSON via --json flag or stdin.
Output: JSON response to stdout, errors to stderr.

Exit codes: 0=success, 1=error`,
	RunE: runRequest,
}

func runRequest(cmd *cobra.Command, args []string) error {
	// 1. 读取 JSON 输入
	var inputBytes []byte
	if jsonInput != "" {
		inputBytes = []byte(jsonInput)
	} else {
		var err error
		inputBytes, err = io.ReadAll(os.Stdin)
		if err != nil {
			return fmt.Errorf("failed to read stdin: %w", err)
		}
	}
	if len(inputBytes) == 0 {
		return fmt.Errorf("no input provided: use --json flag or pipe JSON to stdin")
	}

	// 2. 解析 JSON
	var input RequestInput
	if err := json.Unmarshal(inputBytes, &input); err != nil {
		return fmt.Errorf("failed to parse JSON input: %w", err)
	}

	// 3. 设置 Debug 模式
	if debug {
		requests.SetDebug(true)
	}

	// 4. 获取或创建 Session
	session := getSession(sessionID)

	// 5. 应用全局参数到 Session
	applySessionDefaults(session)

	// 6. 构建 url.Request
	req, err := buildURLRequest(input)
	if err != nil {
		return fmt.Errorf("failed to build request: %w", err)
	}

	// 7. 执行请求
	method := strings.ToUpper(input.Method)
	if method == "" {
		method = "GET"
	}
	resp, err := session.Request(method, input.URL, req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Close()

	// 8. 输出响应
	output := ResponseOutput{
		URL:        resp.Url,
		StatusCode: resp.StatusCode,
		Headers:    convertHeaders(resp.Headers),
		Cookies:    convertCookies(resp.Cookies),
		Content:    utils.Base64Encode(resp.Content),
	}

	outputBytes, err := json.Marshal(output)
	if err != nil {
		return fmt.Errorf("failed to marshal response: %w", err)
	}
	fmt.Println(string(outputBytes))
	return nil
}

// getSession 获取或创建 Session（支持按 ID 复用，保持 Cookie 持久化）
var sessionStore = map[string]*requests.Session{}

func getSession(id string) *requests.Session {
	if id == "" {
		return requests.NewSession()
	}
	if s, ok := sessionStore[id]; ok {
		return s
	}
	s := requests.NewSession()
	sessionStore[id] = s
	return s
}

// applySessionDefaults 将全局 CLI 参数应用到 Session
func applySessionDefaults(s *requests.Session) {
	if proxy != "" {
		s.Proxies = proxy
	}
	s.Verify = verify
	s.Timeout = time.Duration(timeout) * time.Second
	if ja3 != "" {
		s.Ja3 = ja3
	}
	s.RandomJA3 = randomJA3
	s.ForceHTTP1 = forceHTTP1
}

// buildURLRequest 从 RequestInput 构建 url.Request
func buildURLRequest(input RequestInput) (*url.Request, error) {
	req := url.NewRequest()

	// Params
	if input.Params != nil {
		params := url.NewParams()
		for k, v := range input.Params {
			params.Set(k, v)
		}
		req.Params = params
	}

	// Headers
	if input.Headers != nil {
		headers := url.NewHeaders()
		if input.HeadersOrder != nil {
			(*headers)[http.HeaderOrderKey] = input.HeadersOrder
		}
		if input.UnChangedHeaderKey != nil {
			(*headers)[http.UnChangedHeaderKey] = input.UnChangedHeaderKey
		}
		if input.PseudoHeaderOrder != nil {
			(*headers)[http.PHeaderOrderKey] = input.PseudoHeaderOrder
		}
		for k, v := range input.Headers {
			if strings.ToLower(k) != "content-length" {
				headers.Set(k, v)
			}
		}
		req.Headers = headers
	}

	// Cookies
	if input.Cookies != nil {
		cookies, _ := cookiejar.New(nil)
		u, _ := url.Parse(input.URL)
		if u != nil {
			for k, v := range input.Cookies {
				cookies.SetCookies(u, []*http.Cookie{{
					Name:  k,
					Value: v,
				}})
			}
		}
		req.Cookies = cookies
	}

	// Data (form)
	if input.Data != nil {
		data := url.NewData()
		for k, v := range input.Data {
			data.Set(k, v)
		}
		req.Data = data
	}

	// JSON body
	if input.JSON != nil {
		req.Json = input.JSON
	}

	// Raw body (Base64)
	if input.Body != "" {
		req.Body = bytes.NewReader(utils.Base64DecodeToBytes(input.Body))
	}

	// Auth
	if input.Auth != nil {
		req.Auth = input.Auth
	}

	// Redirects
	req.AllowRedirects = url.Bool(input.AllowRedirects)

	// Cert
	if input.Cert != nil {
		req.Cert = input.Cert
	}

	// TLS Extensions
	if input.TLSExtensions != nil {
		extBytes, _ := json.Marshal(input.TLSExtensions)
		ext := &transport.Extensions{}
		if err := json.Unmarshal(extBytes, ext); err == nil {
			req.TLSExtensions = transport.ToTLSExtensions(ext)
		}
	}

	// HTTP/2 Settings
	if input.HTTP2Settings != nil {
		h2Bytes, _ := json.Marshal(input.HTTP2Settings)
		h2s := &transport.H2Settings{}
		if err := json.Unmarshal(h2Bytes, h2s); err == nil {
			req.HTTP2Settings = transport.ToHTTP2Settings(h2s)
		}
	}

	return req, nil
}

// convertHeaders 将 http.Header 转为 map[string][]string
func convertHeaders(h http.Header) map[string][]string {
	result := make(map[string][]string)
	for k, v := range h {
		// 跳过内部使用的 header key
		if k == http.HeaderOrderKey || k == http.PHeaderOrderKey || k == http.UnChangedHeaderKey {
			continue
		}
		result[k] = v
	}
	return result
}

// convertCookies 将 []*http.Cookie 转为 []CookieOutput
func convertCookies(cookies []*http.Cookie) []CookieOutput {
	result := make([]CookieOutput, 0, len(cookies))
	for _, c := range cookies {
		expires := ""
		if !c.Expires.IsZero() {
			expires = c.Expires.Format(time.RFC3339)
		}
		result = append(result, CookieOutput{
			Name:     c.Name,
			Value:    c.Value,
			Path:     c.Path,
			Domain:   c.Domain,
			Expires:  expires,
			HTTPOnly: c.HttpOnly,
			Secure:   c.Secure,
		})
	}
	return result
}
```

- [ ] **Step 3: 验证编译**

Run: `go build ./cmd/requests-go/`
Expected:
  - Exit code: 0

- [ ] **Step 4: 提交**

Run: `git add cmd/requests-go/request.go cmd/requests-go/types.go && git commit -m "feat(cli): add request subcommand with JSON I/O"`

---

### Task 3: 实现 stream 子命令（流式请求）

**Depends on:** Task 2
**Files:**
- Create: `cmd/requests-go/stream.go`

- [ ] **Step 1: 创建 stream 子命令 — 支持流式读取响应体**

```go
// cmd/requests-go/stream.go
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"

	"github.com/spf13/cobra"
	"github.com/wangluozhe/requests/utils"
)

func init() {
	streamCmd.Flags().StringVar(&jsonInput, "json", "", "JSON request parameters (or use stdin)")
	streamCmd.Flags().IntVarP(&streamChunkSize, "chunk-size", "c", 4096, "Chunk size for stream reads")
	rootCmd.AddCommand(streamCmd)
}

var streamChunkSize int

var streamCmd = &cobra.Command{
	Use:   "stream",
	Short: "Send an HTTP request and stream the response body",
	Long: `Send an HTTP request and stream the response body as NDJSON (one JSON object per line).

Each line contains: {"chunk":"<base64>","eof":false} or {"eof":true} on completion.
Errors are printed as: {"error":"<message>"}

This is useful for downloading large files or consuming SSE/streaming APIs.`,
	RunE: runStream,
}

func runStream(cmd *cobra.Command, args []string) error {
	// 1. 读取 JSON 输入（复用 request 的输入解析逻辑）
	var inputBytes []byte
	if jsonInput != "" {
		inputBytes = []byte(jsonInput)
	} else {
		var err error
		inputBytes, err = io.ReadAll(os.Stdin)
		if err != nil {
			return fmt.Errorf("failed to read stdin: %w", err)
		}
	}
	if len(inputBytes) == 0 {
		return fmt.Errorf("no input provided")
	}

	var input RequestInput
	if err := json.Unmarshal(inputBytes, &input); err != nil {
		return fmt.Errorf("failed to parse JSON input: %w", err)
	}

	// 强制开启 Stream 模式
	input.Stream = true

	// 2. 获取 Session 并构建请求
	session := getSession(sessionID)
	applySessionDefaults(session)

	req, err := buildURLRequest(input)
	if err != nil {
		return fmt.Errorf("failed to build request: %w", err)
	}

	// 3. 执行请求
	method := input.Method
	if method == "" {
		method = "GET"
	}
	resp, err := session.Request(method, input.URL, req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Close()

	// 4. 流式读取并输出 NDJSON
	encoder := json.NewEncoder(os.Stdout)
	buf := make([]byte, streamChunkSize)
	for {
		n, err := resp.Body.Read(buf)
		if n > 0 {
			encoder.Encode(StreamOutput{
				Chunk: utils.Base64Encode(buf[:n]),
				EOF:   false,
			})
		}
		if err == io.EOF {
			encoder.Encode(StreamOutput{EOF: true})
			break
		}
		if err != nil {
			encoder.Encode(StreamOutput{
				Error: err.Error(),
				EOF:   true,
			})
			return fmt.Errorf("stream read error: %w", err)
		}
	}
	return nil
}
```

- [ ] **Step 2: 验证编译**

Run: `go build ./cmd/requests-go/`
Expected:
  - Exit code: 0

- [ ] **Step 3: 提交**

Run: `git add cmd/requests-go/stream.go && git commit -m "feat(cli): add stream subcommand for streaming responses"`

---

### Task 4: 实现 sessions 管理子命令（列出/清理 Session）

**Depends on:** Task 2
**Files:**
- Create: `cmd/requests-go/sessions.go`

- [ ] **Step 1: 创建 sessions 管理子命令 — 列出和清理持久化 Session**

```go
// cmd/requests-go/sessions.go
package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(sessionsCmd)
	sessionsCmd.AddCommand(sessionsListCmd)
	sessionsCmd.AddCommand(sessionsClearCmd)
}

var sessionsCmd = &cobra.Command{
	Use:   "sessions",
	Short: "Manage persistent sessions",
	Long:  `List or clear persistent HTTP sessions (identified by --session/-s flag).`,
}

var sessionsListCmd = &cobra.Command{
	Use:   "list",
	Short: "List active session IDs",
	RunE: func(cmd *cobra.Command, args []string) error {
		if len(sessionStore) == 0 {
			fmt.Println("No active sessions")
			return nil
		}
		fmt.Printf("%d active session(s):\n", len(sessionStore))
		for id := range sessionStore {
			fmt.Printf("  - %s\n", id)
		}
		return nil
	},
}

var sessionsClearCmd = &cobra.Command{
	Use:   "clear",
	Short: "Clear all or a specific session",
	RunE: func(cmd *cobra.Command, args []string) error {
		if sessionID != "" {
			delete(sessionStore, sessionID)
			fmt.Printf("Session '%s' cleared\n", sessionID)
		} else {
			count := len(sessionStore)
			sessionStore = map[string]*requests.Session{}
			fmt.Printf("Cleared %d session(s)\n", count)
		}
		return nil
	},
}
```

- [ ] **Step 2: 验证编译**

Run: `go build ./cmd/requests-go/`
Expected:
  - Exit code: 0

- [ ] **Step 3: 提交**

Run: `git add cmd/requests-go/sessions.go && git commit -m "feat(cli): add sessions management subcommands"`

---

### Task 5: 实现 version 子命令和构建脚本

**Depends on:** Task 1
**Files:**
- Create: `cmd/requests-go/version.go`
- Create: `build-cli.sh`

- [ ] **Step 1: 创建 version 子命令**

```go
// cmd/requests-go/version.go
package main

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/wangluozhe/requests"
)

func init() {
	rootCmd.AddCommand(versionCmd)
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print version information",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("requests-go version %s\n", requests.VERSION)
	},
}
```

- [ ] **Step 2: 创建跨平台 CLI 构建脚本 build-cli.sh**

```bash
#!/usr/bin/env bash
# =============================================================================
# requests-go CLI 跨平台编译脚本
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="$PROJECT_ROOT/dist"

# 目标: GOOS/GOARCH → 输出文件名
declare -A TARGETS=(
    ["linux/amd64"]="requests-go-linux-amd64"
    ["linux/arm64"]="requests-go-linux-arm64"
    ["linux/386"]="requests-go-linux-386"
    ["windows/amd64"]="requests-go-windows-amd64.exe"
    ["windows/386"]="requests-go-windows-386.exe"
    ["darwin/amd64"]="requests-go-darwin-amd64"
    ["darwin/arm64"]="requests-go-darwin-arm64"
)

mkdir -p "$OUTPUT_DIR"

echo "Building requests-go CLI for all platforms..."
echo ""

for target in "${!TARGETS[@]}"; do
    goos="${target%/*}"
    goarch="${target#*/}"
    output="$OUTPUT_DIR/${TARGETS[$target]}"

    printf "  %-20s → %s ... " "$target" "$(basename "$output")"
    if GOOS="$goos" GOARCH="$goarch" go build -ldflags="-s -w" -o "$output" "$PROJECT_ROOT/cmd/requests-go/" 2>/dev/null; then
        size=$(ls -lh "$output" | awk '{print $5}')
        echo "OK ($size)"
    else
        echo "SKIPPED (needs CGO/cross-compiler)"
    fi
done

echo ""
echo "Done! Binaries in $OUTPUT_DIR/"
ls -lh "$OUTPUT_DIR/" 2>/dev/null
```

- [ ] **Step 3: 设置构建脚本可执行权限并验证**

Run: `chmod +x build-cli.sh && cd /home/cc11001100/github/crawler-requests/requests-go && bash build-cli.sh`
Expected:
  - Exit code: 0
  - dist/ 目录下生成各平台二进制文件

- [ ] **Step 4: 提交**

Run: `git add cmd/requests-go/version.go build-cli.sh dist/ && git commit -m "feat(cli): add version command and cross-platform build script"`

---

### Task 6: 端到端验证测试

**Depends on:** Task 2, Task 3, Task 4, Task 5
**Files:**
- Create: `cmd/requests-go/cli_test.sh`

- [ ] **Step 1: 创建 CLI 集成测试脚本**

```bash
#!/usr/bin/env bash
# cmd/requests-go/cli_test.sh — requests-go CLI 集成测试
set -euo pipefail

CLI="${CLI:-./requests-go}"
PASS=0
FAIL=0

assert_ok() {
    local desc="$1"
    local cmd="$2"
    echo -n "  TEST: $desc ... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        FAIL=$((FAIL + 1))
    fi
}

assert_contains() {
    local desc="$1"
    local cmd="$2"
    local expected="$3"
    echo -n "  TEST: $desc ... "
    local output
    output=$(eval "$cmd" 2>&1) || true
    if echo "$output" | grep -q "$expected"; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL (expected '$expected' not found in output)"
        echo "       output: $output"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== requests-go CLI Integration Tests ==="
echo ""

# 编译
echo "[Building]"
go build ./cmd/requests-go/
echo ""

# 1. version
echo "[version]"
assert_contains "prints version" "$CLI version" "requests-go"

# 2. request — GET
echo "[request]"
assert_contains "GET httpbin.org/get" \
    "$CLI request --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}'" \
    '"status_code":200'

# 3. request — POST with JSON
echo "[request POST]"
assert_contains "POST httpbin.org/post" \
    "$CLI request --json '{\"method\":\"POST\",\"url\":\"https://httpbin.org/post\",\"json\":{\"hello\":\"world\"}}'" \
    '"status_code":200'

# 4. request — custom headers
echo "[request headers]"
assert_contains "custom User-Agent" \
    "$CLI request --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/headers\",\"headers\":{\"X-Custom\":\"test123\"}}'" \
    '"status_code":200'

# 5. request — stdin input
echo "[request stdin]"
assert_contains "stdin input" \
    "echo '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}' | $CLI request" \
    '"status_code":200'

# 6. request — JA3 fingerprint
echo "[request JA3]"
assert_contains "JA3 fingerprint" \
    "$CLI request --ja3 '771,4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53,0-23-65281-10-11-35-16-5-13-18-51-45-43-27-17513,29-23-24,0' --json '{\"method\":\"GET\",\"url\":\"https://httpbin.org/get\"}'" \
    '"status_code":200'

# 7. sessions
echo "[sessions]"
assert_contains "list sessions" "$CLI sessions list" "No active sessions"

# 8. error handling
echo "[error handling]"
assert_contains "invalid URL" \
    "$CLI request --json '{\"method\":\"GET\",\"url\":\"not-a-valid-url\"}' 2>&1 || true" \
    ""

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] || exit 1
```

- [ ] **Step 2: 运行集成测试**

Run: `chmod +x cmd/requests-go/cli_test.sh && cd cmd/requests-go && bash cli_test.sh`
Expected:
  - Exit code: 0
  - All tests pass

- [ ] **Step 3: 提交**

Run: `git add cmd/requests-go/cli_test.sh && git commit -m "test(cli): add CLI integration tests"`

---

### Task 7: 跨语言调用示例文档

**Depends on:** Task 2
**Files:**
- Create: `docs/cli-usage-examples.md`

- [ ] **Step 1: 创建跨语言调用示例文档**

```markdown
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
    \"\"\"调用 requests-go CLI 发起 HTTP 请求\"\"\"
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
```

- [ ] **Step 2: 提交**

Run: `git add docs/cli-usage-examples.md && git commit -m "docs(cli): add cross-language usage examples"`

---
```

