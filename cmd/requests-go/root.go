package main

import (
	"github.com/spf13/cobra"
)

var (
	sessionID  string
	proxy      string
	verify     bool
	timeout    int
	ja3        string
	randomJA3  bool
	forceHTTP1 bool
	debug      bool
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
