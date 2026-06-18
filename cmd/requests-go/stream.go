package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"

	"github.com/spf13/cobra"
	"github.com/wangluozhe/requests/utils"
)

var streamChunkSize int

func init() {
	streamCmd.Flags().StringVar(&jsonInput, "json", "", "JSON request parameters (or use stdin)")
	streamCmd.Flags().IntVarP(&streamChunkSize, "chunk-size", "c", 4096, "Chunk size for stream reads")
	rootCmd.AddCommand(streamCmd)
}

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
		return fmt.Errorf("no input provided")
	}

	var input RequestInput
	if err := json.Unmarshal(inputBytes, &input); err != nil {
		return fmt.Errorf("failed to parse JSON input: %w", err)
	}

	// Force Stream mode
	input.Stream = true

	// 2. Get Session and build request
	session := getSession(sessionID)
	applySessionDefaults(session)

	req, err := buildURLRequest(input)
	if err != nil {
		return fmt.Errorf("failed to build request: %w", err)
	}

	// 3. Execute request
	method := input.Method
	if method == "" {
		method = "GET"
	}
	resp, err := session.Request(method, input.URL, req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Close()

	// 4. Stream read and output NDJSON
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
