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
	Body               string            `json:"body,omitempty"`        // Base64 encoded
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
	Content    string              `json:"content,omitempty"` // Base64 encoded
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
	Chunk string `json:"chunk,omitempty"` // Base64 encoded data chunk
	EOF   bool   `json:"eof"`
	Error string `json:"error,omitempty"`
}
