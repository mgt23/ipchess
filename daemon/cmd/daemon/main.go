package main

import (
	"context"
	"ipchess/p2p"
	"os"
	"os/signal"
	"time"

	"github.com/libp2p/go-libp2p-core/peer"
	"go.uber.org/zap"
)

func main() {
	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	ctx, ctxCancel := context.WithCancel(context.Background())

	go func() {
		sigChan := make(chan os.Signal, 1)
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
	logger.Info("started node", zap.String("ID", h.ID().Pretty()))

	for !h.Connected() {
		<-time.After(10 * time.Millisecond)
	}
	logger.Info("node online")

	if len(os.Args) > 1 {
		switch os.Args[1] {
		case "challenge":
			peerID, err := peer.Decode(os.Args[2])
			if err != nil {
				panic(err)
			}

			if err := h.ChallengePeer(ctx, peerID); err != nil {
				panic(err)
			}
		}
	}

	<-ctx.Done()
}
