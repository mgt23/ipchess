package main

import (
	"context"
	"phessdaemon/p2p"
)

func main() {
	h := p2p.NewHost()
	if err := h.Start(context.Background()); err != nil {
		panic(err)
	}
	defer h.Close()
}
