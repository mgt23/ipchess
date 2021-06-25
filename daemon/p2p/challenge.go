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

// ChallengeDeclinedError indicates that a challenge was not accepted by us or the peer for some reason.
type ChallengeDeclinedError struct {
	Reason ChallengeDeclinedReason
}

func (err *ChallengeDeclinedError) Error() string {
	return fmt.Sprintf("match challenged declined: %s", err.Reason)
}

// ChallengeDeclinedReason enumerates the possible challenge declination reasons.
type ChallengeDeclinedReason string

var (
	DeclinedByPeer           ChallengeDeclinedReason = "peer declined the challenge"
	InvalidRandomBytesLength ChallengeDeclinedReason = "received more than one random byte from peer"
	CommitmentMismatch       ChallengeDeclinedReason = "peer preimage does not match commitment"
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
func (c *challenge) Initiate(ctx context.Context, stream network.Stream) (*MatchInfo, error) {
	c.logger.Debug("generating challenge request random bytes")
	rb := make([]byte, 32)
	if _, err := rand.Read(rb); err != nil {
		return nil, err
	}

	c.logger.Debug("generating challenge challenge request commitment")
	commitment, err := multihash.Encode(rb, multihash.SHA2_256)
	if err != nil {
		return nil, err
	}

	c.logger.Debug("sending challenge request")
	challengeRequest := &ipchessproto.ChallengeRequest{
		Commitment: commitment,
	}
	if err := sendMessage(ctx, stream, challengeRequest); err != nil {
		return nil, err
	}

	c.logger.Debug("waiting challenge request response")
	var challengeRequestResponse ipchessproto.ChallengeRequestResponse
	if err := receiveMessage(ctx, stream, &challengeRequestResponse); err != nil {
		return nil, err
	}

	if len(challengeRequestResponse.RandomBytes) == 0 {
		return nil, &ChallengeDeclinedError{Reason: DeclinedByPeer}
	} else if len(challengeRequestResponse.RandomBytes) != 32 {
		return nil, &ChallengeDeclinedError{Reason: InvalidRandomBytesLength}
	}

	c.logger.Debug("sending challenge commitment preimage")
	commitmentPreimage := &ipchessproto.ChallengeCommitmentPreimage{
		Preimage: rb,
	}
	if err := sendMessage(ctx, stream, commitmentPreimage); err != nil {
		return nil, err
	}

	m := &MatchInfo{}

	for i := 0; i < 32; i++ {
		m.ID[i] = rb[i] ^ challengeRequestResponse.RandomBytes[i]
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
func (c *challenge) Handle(ctx context.Context, stream network.Stream) (*MatchInfo, error) {
	c.logger.Debug("waiting challenge request")
	var challengeRequest ipchessproto.ChallengeRequest
	if err := receiveMessage(ctx, stream, &challengeRequest); err != nil {
		return nil, err
	}

	c.logger.Debug("generating challenge request response random bytes")
	rb := make([]byte, 32)
	if _, err := rand.Read(rb); err != nil {
		return nil, err
	}

	c.logger.Debug("sending challenge request response")
	challengeRequestResponse := &ipchessproto.ChallengeRequestResponse{
		RandomBytes: rb,
	}
	if err := sendMessage(ctx, stream, challengeRequestResponse); err != nil {
		return nil, err
	}

	c.logger.Debug("waiting challenge commitment preimage")
	var commitmentPreimage ipchessproto.ChallengeCommitmentPreimage
	if err := receiveMessage(ctx, stream, &commitmentPreimage); err != nil {
		return nil, err
	}

	c.logger.Debug("checking piece color negotiation preimage")
	hashedPreimage, err := multihash.Encode(commitmentPreimage.Preimage, multihash.SHA2_256)
	if err != nil {
		return nil, err
	}

	if !bytes.Equal(hashedPreimage, challengeRequest.Commitment) {
		return nil, &ChallengeDeclinedError{Reason: CommitmentMismatch}
	}

	m := &MatchInfo{}

	for i := 0; i < 32; i++ {
		m.ID[i] = rb[i] ^ commitmentPreimage.Preimage[i]
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
