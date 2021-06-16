package p2p

import (
	"context"
	"errors"
	"time"

	"github.com/libp2p/go-libp2p"
	"github.com/libp2p/go-libp2p-core/host"
	"github.com/libp2p/go-libp2p-core/network"
	"github.com/libp2p/go-libp2p-core/peer"
	"github.com/libp2p/go-libp2p-kad-dht"
	"go.uber.org/zap"
)

var (
	alreadyInMatchError = errors.New("host is already in a match")
)

type HostOption func(*Host)

func WithLogger(logger *zap.Logger) HostOption {
	return func(h *Host) {
		h.logger = logger
	}
}

// Host is responsible for handling the protocol steps with peers.
type Host struct {
	mailbox chan interface{}
	done    chan struct{}
	started bool

	p2pHost host.Host
	kadDHT  *dht.IpfsDHT

	currentMatch *Match

	logger *zap.Logger
}

// NewHost creates a new host which can be started later.
func NewHost(options ...HostOption) *Host {
	h := &Host{
		mailbox: make(chan interface{}),
		done:    make(chan struct{}),
		logger:  zap.NewNop(),
	}

	for _, option := range options {
		option(h)
	}

	go h.background()

	return h
}

// Close stops the host's networking processes and background loop.
func (h *Host) Close() {
	close(h.mailbox)
	<-h.done
}

// ID returns the host's peer ID.
func (h *Host) ID() peer.ID {
	return h.p2pHost.ID()
}

// Start signals the host to start the protocol network processes.
func (h *Host) Start(ctx context.Context) error {
	msg := &hostStart{
		Ctx: ctx,
		Err: make(chan error),
	}
	h.mailbox <- msg

	return <-msg.Err
}

// ChallengePeer signals the host to look for the match with the given CID.
func (h *Host) ChallengePeer(ctx context.Context, peerID peer.ID) error {
	msg := &hostChallengePeer{
		Ctx:    ctx,
		Err:    make(chan error),
		PeerID: peerID,
	}
	h.mailbox <- msg

	return <-msg.Err
}

func (h *Host) background() {
	defer close(h.done)

	for {
		msg, ok := <-h.mailbox
		if !ok {
			break
		}

		switch msg := msg.(type) {
		case *hostStart:
			msg.Err <- h.start(msg.Ctx)
			close(msg.Err)

		case *hostChallengePeer:
			msg.Err <- h.challengePeer(msg.Ctx, msg.PeerID)
			close(msg.Err)

		default:
			panic("invalid message type for Host")
		}
	}

	if h.started {
		h.kadDHT.Close()
		h.p2pHost.Close()
	}
}

func (h *Host) start(ctx context.Context) error {
	p2pHost, err := libp2p.New(ctx)
	if err != nil {
		return err
	}

	h.p2pHost = p2pHost
	h.p2pHost.SetStreamHandler(ipchessProtocolID, h.handleStream)

	kadDHT, err := dht.New(
		ctx,
		h.p2pHost,
		dht.BootstrapPeers(dht.GetDefaultBootstrapPeerAddrInfos()...),
	)
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

func (h *Host) challengePeer(ctx context.Context, peerID peer.ID) error {
	if h.currentMatch != nil {
		return alreadyInMatchError
	}

	go func() {
		for {
			if h.hasDHTPeers() {
				peerAddrInfo, err := h.kadDHT.FindPeer(ctx, peerID)
				if err != nil {
					h.logger.Error("failed finding peer", zap.Error(err))
				} else {
					h.logger.Debug("peer found", zap.String("addrInfo", peerAddrInfo.String()))

					s, _ := h.p2pHost.NewStream(ctx, peerAddrInfo.ID, ipchessProtocolID)
					s.Close()
				}
			}

			select {
			case <-ctx.Done():
				return
			case <-time.After(time.Second):
			}
		}
	}()

	return nil
}

func (h *Host) handleStream(stream network.Stream) {
	h.logger.Debug("new peer stream", zap.String("peerID", stream.Conn().RemotePeer().Pretty()))
	stream.Close()
}

func (h *Host) hasDHTPeers() bool {
	for _, conn := range h.p2pHost.Network().Conns() {
		s, _ := h.p2pHost.Peerstore().FirstSupportedProtocol(conn.RemotePeer(), string(dht.ProtocolDHT))
		if s != "" {
			return true
		}
	}

	return false
}

type hostStart struct {
	Ctx context.Context
	Err chan error
}

type hostChallengePeer struct {
	Ctx context.Context
	Err chan error

	PeerID peer.ID
}
