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

			match, err := h.ChallengePeer(ctx, peerID)
			if err != nil {
				panic(err)
			}

			move, err := match.ReceiveMove(ctx)
			if err != nil {
				panic(err)
			}
			logger.Debug("received move", zap.Any("move", move))
		}
	} else {
		match, err := h.Accept(ctx)
		if err == context.Canceled {
		} else if err != nil {
			panic(err)
		}

		if match != nil {
			move := p2p.Move{
				FromRow: 1,
				FromCol: 2,
				ToRow:   3,
				ToCol:   4,
			}
			if err := match.SendMove(context.Background(), move); err != nil {
				logger.Error("failed sending move", zap.Error(err))
			}
			logger.Debug("sent move", zap.Any("move", move))
		}
	}

	<-ctx.Done()
}
