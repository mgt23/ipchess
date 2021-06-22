package commands

import (
	"context"
	"ipchess/api"
	"ipchess/p2p"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// NewDaemonCmd creates a new daemon command.
func NewDaemonCmd() *cobra.Command {
	var apiPort uint16
	var debug bool

	cmd := &cobra.Command{
		Use:   "daemon",
		Short: "Start IPChess daemon process",
		Run: func(cmd *cobra.Command, args []string) {
			loggerCfg := zap.Config{
				Level:            zap.NewAtomicLevelAt(zapcore.InfoLevel),
				Encoding:         "console",
				OutputPaths:      []string{"stdout"},
				ErrorOutputPaths: []string{"stderr"},
				EncoderConfig: zapcore.EncoderConfig{
					LevelKey:    "level",
					EncodeLevel: zapcore.CapitalColorLevelEncoder,
					TimeKey:     "ts",
					EncodeTime:  zapcore.ISO8601TimeEncoder,
					MessageKey:  "msg",
				},
			}
			if debug {
				loggerCfg.Development = true
				loggerCfg.Level = zap.NewAtomicLevelAt(zapcore.DebugLevel)
			}

			logger, err := loggerCfg.Build()
			if err != nil {
				panic(err)
			}

			ctx, ctxCancel := context.WithCancel(context.Background())

			go func() {
				sigChan := make(chan os.Signal, 1)
				signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
				<-sigChan
				logger.Info("shutting down...")
				signal.Reset(syscall.SIGINT, syscall.SIGTERM)

				ctxCancel()
			}()

			h := p2p.NewHost(p2p.WithLogger(logger))
			if err := h.Start(ctx); err != nil {
				panic(err)
			}
			defer h.Close()
			logger.Info("started node", zap.String("ID", h.ID().Pretty()))

		connectLoop:
			for {
				select {
				case <-time.After(10 * time.Millisecond):
					if h.Connected() {
						break connectLoop
					}
				case <-ctx.Done():
					return
				}
			}
			logger.Info("node online")

			apiHandler := api.NewHandler(h, api.WithLogger(logger))
			logger.Info("starting local API", zap.Uint16("port", apiPort))
			apiHandler.StartLocal(apiPort)
			defer apiHandler.Shutdown()

			<-ctx.Done()
		},
	}

	cmd.Flags().Uint16Var(&apiPort, "api.port", 0, "API port")
	cmd.Flags().BoolVar(&debug, "debug", false, "Turn on debugging logs")
	cmd.MarkFlagRequired("api.port")

	return cmd
}
