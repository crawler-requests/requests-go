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
