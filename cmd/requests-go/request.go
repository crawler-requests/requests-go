package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	neturl "net/url"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"
	http "github.com/wangluozhe/chttp"
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
	// 1. Read JSON input
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

	// 2. Parse JSON
	var input RequestInput
	if err := json.Unmarshal(inputBytes, &input); err != nil {
		return fmt.Errorf("failed to parse JSON input: %w", err)
	}

	// 3. Set debug mode
	if debug {
		requests.SetDebug(true)
	}

	// 4. Get or create Session
	session := getSession(sessionID)

	// 5. Apply global flags to Session
	applySessionDefaults(session)

	// 6. Build url.Request
	req, err := buildURLRequest(input)
	if err != nil {
		return fmt.Errorf("failed to build request: %w", err)
	}

	// 7. Execute request
	method := strings.ToUpper(input.Method)
	if method == "" {
		method = "GET"
	}
	resp, err := session.Request(method, input.URL, req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Close()

	// 8. Output response
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

	// Cookies — assign map directly; the library's GetCookies handles map[string]string
	if input.Cookies != nil {
		cookies, _ := cookiejar.New(nil)
		u, _ := neturl.Parse(input.URL)
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
		// Skip internal header keys
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
