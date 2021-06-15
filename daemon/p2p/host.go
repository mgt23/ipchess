package p2p

import (
	"context"
	"errors"
	"time"

	"github.com/ipfs/go-cid"
	"github.com/libp2p/go-libp2p"
	"github.com/libp2p/go-libp2p-core/host"
	"github.com/libp2p/go-libp2p-core/network"
	"github.com/libp2p/go-libp2p-kad-dht"
	kbucket "github.com/libp2p/go-libp2p-kbucket"
	"go.uber.org/zap"
)

var (
	alreadyProvidingError = errors.New("host is already providing a match")
)

type HostOption func(*Host)

func WithLogger(logger *zap.Logger) HostOption {
	return func(h *Host) {
		h.logger = logger
	}
}

// Host is responsible for handling the Phess protocol steps with peers.
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

// Start signals the host to start the Phess protocol network processes.
func (h *Host) Start(ctx context.Context) error {
	msg := &hostStart{
		Ctx: ctx,
		Err: make(chan error),
	}
	h.mailbox <- msg

	return <-msg.Err
}

// ProvideMatch signals the host to start providing a new match. After the first peer looking for the provided match CID
// will be accepted.
func (h *Host) ProvideMatch(ctx context.Context) error {
	msg := &hostProvideMatch{
		Ctx: ctx,
		Err: make(chan error),
	}
	h.mailbox <- msg

	return <-msg.Err
}

// JoinMatch signals the host to look for the match with the given CID.
func (h *Host) JoinMatch(ctx context.Context, matchCID cid.Cid) error {
	return nil
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

		case *hostProvideMatch:
			msg.Err <- h.provideMatch(msg.Ctx)
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

func (h *Host) provideMatch(ctx context.Context) error {
	if h.currentMatch != nil {
		return alreadyProvidingError
	}

	h.currentMatch = newMatch(h.p2pHost.ID())
	matchCID := h.currentMatch.CID

	go func() {
	loop:
		for {
			h.logger.Debug("providing match", zap.String("CID", matchCID.String()))

			provideCtx, cancelProvideCtx := context.WithTimeout(ctx, time.Second)
			err := h.kadDHT.Provide(provideCtx, matchCID, true)
			cancelProvideCtx()
			if err == kbucket.ErrLookupFailure {
				// We have no peers yet. Ignore the error to prevent flooding the logs.
				<-time.After(5 * time.Second)
			} else if err != nil {
				h.logger.Error("failed providing match", zap.Error(err))
			}

			select {
			case <-ctx.Done():
				break loop
			default:
			}
		}
	}()

	return nil
}

func (h *Host) handleStream(stream network.Stream) {
}

type hostStart struct {
	Ctx context.Context
	Err chan error
}

type hostProvideMatch struct {
	Ctx context.Context
	Err chan error
}
