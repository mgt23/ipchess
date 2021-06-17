package p2p

import (
	"context"
	"errors"
	"sync"
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
//
// thread-safe.
type Host struct {
	stateLock sync.RWMutex

	p2pHost host.Host
	kadDHT  *dht.IpfsDHT

	currentMatch *Match

	logger *zap.Logger
}

// NewHost creates a new host which can be started later.
func NewHost(options ...HostOption) *Host {
	h := &Host{
		logger: zap.NewNop(),
	}

	for _, option := range options {
		option(h)
	}

	return h
}

// Close stops the host's networking processes and background loop.
func (h *Host) Close() {
	if h.p2pHost != nil {
		h.p2pHost.Close()
	}

	if h.kadDHT != nil {
		h.kadDHT.Close()
	}
}

// ID returns the host's peer ID.
func (h *Host) ID() peer.ID {
	return h.p2pHost.ID()
}

// Start signals the host to start the protocol network processes.
func (h *Host) Start(ctx context.Context) error {
	h.stateLock.Lock()
	defer h.stateLock.Unlock()

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
	return nil
}

// ChallengePeer challenges a peer to a match.
func (h *Host) ChallengePeer(ctx context.Context, peerID peer.ID) error {
	h.stateLock.RLock()
	defer h.stateLock.RUnlock()
	if h.currentMatch != nil {
		return alreadyInMatchError
	}

	for {
		if h.hasDHTPeers() {
			peerAddrInfo, err := h.kadDHT.FindPeer(ctx, peerID)
			if err != nil {
				return err
			}

			h.logger.Debug("peer found", zap.String("addrInfo", peerAddrInfo.String()))

			s, _ := h.p2pHost.NewStream(ctx, peerAddrInfo.ID, ipchessProtocolID)
			s.Close()
		}

		select {
		case <-ctx.Done():
			return nil
		case <-time.After(time.Second):
		}
	}
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
