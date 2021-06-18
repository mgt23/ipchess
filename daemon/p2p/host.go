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
	dht "github.com/libp2p/go-libp2p-kad-dht"
	"go.uber.org/zap"
)

var (
	errAlreadyInMatch = errors.New("host is already in a match")
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

// Connected returns true if the host is connected to at least 1 peer that supports the DHT protocol.
func (h *Host) Connected() bool {
	for _, conn := range h.p2pHost.Network().Conns() {
		s, _ := h.p2pHost.Peerstore().FirstSupportedProtocol(conn.RemotePeer(), string(dht.ProtocolDHT))
		if s != "" {
			return true
		}
	}

	return false
}

// ChallengePeer challenges a peer to a match.
func (h *Host) ChallengePeer(ctx context.Context, peerID peer.ID) error {
	h.stateLock.RLock()
	defer h.stateLock.RUnlock()
	if h.currentMatch != nil {
		return errAlreadyInMatch
	}

	for {
		if h.Connected() {
			logger := h.logger.With(zap.String("peerID", peerID.Pretty()))
			logger.Debug("looking for peer")
			peerAddrInfo, err := h.kadDHT.FindPeer(ctx, peerID)
			if err != nil {
				return err
			}

			logger.Debug("peer found", zap.String("addrInfo", peerAddrInfo.String()))

			stream, err := h.p2pHost.NewStream(ctx, peerAddrInfo.ID, ipchessProtocolID)
			if err != nil {
				return err
			}

			err = func() error {
				defer stream.Close()

				c := newChallenge(logger)
				match, err := c.Initiate(ctx, stream)
				if err != nil {
					return err
				}
				logger.Debug("challenge accepted", zap.Any("match", match))

				return nil
			}()
			if err != nil {
				return err
			}
		}

		select {
		case <-ctx.Done():
			return nil
		case <-time.After(time.Second):
		}
	}
}

func (h *Host) handleStream(stream network.Stream) {
	defer stream.Close()

	logger := h.logger.With(zap.String("peerID", stream.Conn().RemotePeer().Pretty()))
	logger.Debug("new peer stream")

	c := newChallenge(logger)
	match, err := c.Handle(context.Background(), stream)
	if err != nil {
		logger.Error("failed handling peer challenge", zap.Error(err))
	}
	logger.Debug("challenge accepted", zap.Any("match", match))
}
