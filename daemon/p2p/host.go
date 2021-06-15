package p2p

import (
	"context"

	"github.com/libp2p/go-libp2p"
	"github.com/libp2p/go-libp2p-core/host"
	"github.com/libp2p/go-libp2p-core/network"
	"github.com/libp2p/go-libp2p-kad-dht"
)

// Host is responsible for handling the Phess protocol steps with peers.
type Host struct {
	mailbox chan interface{}
	done    chan struct{}
	started bool

	p2pHost host.Host
	kadDHT  *dht.IpfsDHT

	currentMatch *Match
}

// NewHost creates a new host which can be started later.
func NewHost() *Host {
	h := &Host{
		mailbox: make(chan interface{}),
		done:    make(chan struct{}),
	}
	go h.background()

	return h
}

// Close stops the host's networking processes and background loop.
func (h *Host) Close() {
	close(h.mailbox)
	<-h.done
}

// Start signals the host to start the Phess protocol network processes.
func (h *Host) Start(ctx context.Context) error {
	msg := &hostStart{
		ctx:   ctx,
		Error: make(chan error),
	}
	h.mailbox <- msg

	return <-msg.Error
}

// background mailbox loop process.
func (h *Host) background() {
	defer close(h.done)

	for {
		msg, ok := <-h.mailbox
		if !ok {
			break
		}

		switch msg := msg.(type) {
		case *hostStart:
			msg.Error <- h.start(msg.ctx)

		default:
			panic("invalid message type for Host")
		}
	}

	if h.started {
		h.kadDHT.Close()
		h.p2pHost.Close()
	}
}

// start starts the host and its DHT client.
func (h *Host) start(ctx context.Context) error {
	p2pHost, err := libp2p.New(ctx)
	if err != nil {
		return err
	}

	h.p2pHost = p2pHost
	h.p2pHost.SetStreamHandler(phessProtocolID, h.handleStream)

	kadDHT, err := dht.New(ctx, h.p2pHost, dht.BootstrapPeers(dht.GetDefaultBootstrapPeerAddrInfos()...))
	if err != nil {
		return err
	}

	if err := kadDHT.Bootstrap(ctx); err != nil {
		return err
	}

	h.kadDHT = kadDHT

	h.started = true
	return nil
}

// handleStream handles opened streams from libp2p peers.
func (h *Host) handleStream(stream network.Stream) {
}

// hostStart holds data for host start message.
type hostStart struct {
	ctx   context.Context
	Error chan error
}
