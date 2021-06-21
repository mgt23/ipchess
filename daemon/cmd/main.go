package main

import (
	"fmt"
	"ipchess/cmd/commands"
	"os"

	"github.com/spf13/cobra"
)

func main() {
	rootCmd := cobra.Command{
		Use: "ipchess",
	}

	rootCmd.AddCommand(commands.NewDaemonCmd())

	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
