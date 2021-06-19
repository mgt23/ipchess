package p2p

import (
	"context"
	"encoding/hex"
	"sync"
	"time"

	"github.com/libp2p/go-libp2p"
	"github.com/libp2p/go-libp2p-core/host"
	"github.com/libp2p/go-libp2p-core/network"
	"github.com/libp2p/go-libp2p-core/peer"
	dht "github.com/libp2p/go-libp2p-kad-dht"
	"go.uber.org/zap"
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

	acceptChan chan *acceptInfo

	logger *zap.Logger
}

// NewHost creates a new host which can be started later.
func NewHost(options ...HostOption) *Host {
	h := &Host{
		logger:     zap.NewNop(),
		acceptChan: make(chan *acceptInfo),
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
			return h.kadDHT.RoutingTable().Size() > 0
		}
	}

	return false
}

// Accept blocks until a challenge is accepted.
// The host will decline incoming challenges that arrive while we are not accepting.
func (h *Host) Accept(ctx context.Context) (*Match, error) {
	ai := &acceptInfo{
		Ctx:   ctx,
		Match: make(chan *Match),
		Err:   make(chan error),
	}
	h.acceptChan <- ai

	select {
	case match := <-ai.Match:
		return match, nil
	case err := <-ai.Err:
		return nil, err
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

// ChallengePeer challenges a peer to a match.
func (h *Host) ChallengePeer(ctx context.Context, peerID peer.ID) (*Match, error) {
	for {
		if h.Connected() {
			logger := h.logger.With(zap.String("peerID", peerID.Pretty()))
			logger.Debug("looking for peer")
			peerAddrInfo, err := h.kadDHT.FindPeer(ctx, peerID)
			if err != nil {
				return nil, err
			}

			logger.Debug("peer found", zap.String("addrInfo", peerAddrInfo.String()))

			stream, err := h.p2pHost.NewStream(ctx, peerAddrInfo.ID, ipchessProtocolID)
			if err != nil {
				return nil, err
			}

			c := newChallenge(logger)
			matchInfo, err := c.Initiate(ctx, stream)
			if err != nil {
				return nil, err
			}
			logger.Debug("challenge accepted", zap.Any("matchInfo", matchInfo))

			logger = logger.With(zap.String("matchID", hex.EncodeToString(matchInfo.ID[:])))
			return newMatch(logger, stream, *matchInfo), nil
		}

		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-time.After(time.Second):
		}
	}
}

func (h *Host) handleStream(stream network.Stream) {
	select {
	case ai := <-h.acceptChan:
		defer func() {
			close(ai.Match)
			close(ai.Err)
		}()

		logger := h.logger.With(zap.String("peerID", stream.Conn().RemotePeer().Pretty()))
		logger.Debug("new peer stream")

		c := newChallenge(logger)
		matchInfo, err := c.Handle(ai.Ctx, stream)
		if err != nil {
			ai.Err <- err
			return
		}
		logger.Debug("challenge accepted", zap.Any("matchInfo", matchInfo))

		logger = logger.With(zap.String("matchID", hex.EncodeToString(matchInfo.ID[:])))
		ai.Match <- newMatch(logger, stream, *matchInfo)
	default:
		// close the stream since we are not accepting challenges
		stream.Close()
	}
}

// acceptInfo holds data for accepting challenges asynchronously.
type acceptInfo struct {
	Ctx   context.Context
	Match chan *Match
	Err   chan error
}
