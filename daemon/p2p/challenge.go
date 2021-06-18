package p2p

import (
	"bytes"
	"context"
	"crypto/rand"
	"fmt"
	"ipchess/gen/ipchessproto"

	"github.com/libp2p/go-libp2p-core/network"
	"github.com/multiformats/go-multihash"
	"go.uber.org/zap"
)

// ChallengeRejectedError indicates that a challenge was not accepted by us or the peer for some reason.
type ChallengeRejectedError struct {
	Reason ChallengeRejectedReason
}

func (err *ChallengeRejectedError) Error() string {
	return fmt.Sprintf("match rejected: %s", err.Reason)
}

// ChallengeRejectedReason enumerates the possible challenge rejection reasons.
type ChallengeRejectedReason string

var (
	PeerReject               ChallengeRejectedReason = "peer did not accept the challenge"
	InvalidRandomBytesLength ChallengeRejectedReason = "received more than one random byte from peer"
	CommitmentMismatch       ChallengeRejectedReason = "peer preimage does not match commitment"
)

// challenge initiates or handles match requests to and from peers.
type challenge struct {
	logger *zap.Logger
}

func newChallenge(logger *zap.Logger) *challenge {
	return &challenge{
		logger: logger,
	}
}

// Initiate challenges a peer to a match.
func (c *challenge) Initiate(ctx context.Context, stream network.Stream) (*Match, error) {
	c.logger.Debug("generating piece color negotiation random bytes")
	rb := make([]byte, 32)
	if _, err := rand.Read(rb); err != nil {
		return nil, err
	}

	c.logger.Debug("generating challenge piece color negotiation commitment")
	commitment, err := multihash.Encode(rb, multihash.SHA2_256)
	if err != nil {
		return nil, err
	}

	c.logger.Debug("sending challenge request")
	challengeReq := &ipchessproto.ChallengeRequest{
		PieceColorNegotiationCommitment: commitment,
	}
	if err := sendMessage(ctx, stream, challengeReq); err != nil {
		return nil, err
	}

	c.logger.Debug("waiting challenge response")
	var challengeRes ipchessproto.ChallengeResponse
	if err := receiveMessage(ctx, stream, &challengeRes); err != nil {
		return nil, err
	}

	if len(challengeRes.PieceColorNegotiationRandomBytes) == 0 {
		return nil, &ChallengeRejectedError{Reason: PeerReject}
	} else if len(challengeRes.PieceColorNegotiationRandomBytes) != 32 {
		return nil, &ChallengeRejectedError{Reason: InvalidRandomBytesLength}
	}

	c.logger.Debug("sending challenge piece color negotiation commitment preimage")
	pcnPreimg := &ipchessproto.PieceColorNegotiationPreimage{
		Preimage: rb,
	}
	if err := sendMessage(ctx, stream, pcnPreimg); err != nil {
		return nil, err
	}

	m := &Match{}

	for i := 0; i < 32; i++ {
		m.ID[i] = rb[i] ^ challengeRes.PieceColorNegotiationRandomBytes[i]
	}

	if (m.ID[0] & 1) == 0 {
		m.White = stream.Conn().LocalPeer()
		m.Black = stream.Conn().RemotePeer()
	} else {
		m.White = stream.Conn().RemotePeer()
		m.Black = stream.Conn().LocalPeer()
	}

	return m, nil
}

// Handle handles an incoming match challenge from a peer.
func (c *challenge) Handle(ctx context.Context, stream network.Stream) (*Match, error) {
	c.logger.Debug("waiting challenge request")
	var challengeReq ipchessproto.ChallengeRequest
	if err := receiveMessage(ctx, stream, &challengeReq); err != nil {
		return nil, err
	}

	c.logger.Debug("generating piece color negotiation random bytes")
	rb := make([]byte, 32)
	if _, err := rand.Read(rb); err != nil {
		return nil, err
	}

	c.logger.Debug("sending challenge response")
	challengeRes := &ipchessproto.ChallengeResponse{
		PieceColorNegotiationRandomBytes: rb,
	}
	if err := sendMessage(ctx, stream, challengeRes); err != nil {
		return nil, err
	}

	c.logger.Debug("waiting piece color negotiation preimage")
	var pcnPreimg ipchessproto.PieceColorNegotiationPreimage
	if err := receiveMessage(ctx, stream, &pcnPreimg); err != nil {
		return nil, err
	}

	c.logger.Debug("checking piece color negotiation preimage")
	preimgHash, err := multihash.Encode(pcnPreimg.Preimage, multihash.SHA2_256)
	if err != nil {
		return nil, err
	}

	if !bytes.Equal(preimgHash, challengeReq.PieceColorNegotiationCommitment) {
		return nil, &ChallengeRejectedError{Reason: CommitmentMismatch}
	}

	m := &Match{}

	for i := 0; i < 32; i++ {
		m.ID[i] = rb[i] ^ pcnPreimg.Preimage[i]
	}

	if (m.ID[0] & 1) == 1 {
		m.White = stream.Conn().LocalPeer()
		m.Black = stream.Conn().RemotePeer()
	} else {
		m.White = stream.Conn().RemotePeer()
		m.Black = stream.Conn().LocalPeer()
	}

	return m, nil
}
