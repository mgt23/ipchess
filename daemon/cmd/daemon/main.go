package main

import (
	"context"
	"os"
	"os/signal"
	"phessdaemon/p2p"

	"go.uber.org/zap"
)

func main() {
	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	ctx, ctxCancel := context.WithCancel(context.Background())

	go func() {
		sigChan := make(chan os.Signal)
		signal.Notify(sigChan, os.Interrupt)
		<-sigChan
		signal.Reset(os.Interrupt)

		ctxCancel()
	}()

	h := p2p.NewHost(p2p.WithLogger(logger))
	if err := h.Start(ctx); err != nil {
		panic(err)
	}
	defer h.Close()

	if err := h.ProvideMatch(ctx); err != nil {
		panic(err)
	}

	<-ctx.Done()
}
