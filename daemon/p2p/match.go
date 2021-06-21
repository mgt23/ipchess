package p2p

import (
	"bytes"
	"context"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"ipchess/gen/ipchessproto"

	"github.com/libp2p/go-libp2p-core/network"
	"github.com/libp2p/go-libp2p-core/peer"
	"go.uber.org/zap"
)

var (
	errInvalidMoveSignature = errors.New("received invalid move signature")
)

type MatchID [32]byte

func (mid MatchID) Pretty() string {
	return hex.EncodeToString(mid[:])
}

// MatchInfo holds data about a InterPlanetary Chess Match.
type MatchInfo struct {
	ID    MatchID
	White peer.ID
	Black peer.ID
}

// Move represents a move to be sent to and received from peers.
type Move struct {
	FromRow uint8
	FromCol uint8
	ToRow   uint8
	ToCol   uint8

	SeqNum uint32
}

// Match handles ongoing IPChess Match events like sending moves, receiving opponent's moves, etc.
type Match struct {
	info   MatchInfo
	logger *zap.Logger
	stream network.Stream
}

func newMatch(logger *zap.Logger, stream network.Stream, info MatchInfo) *Match {
	return &Match{
		info:   info,
		logger: logger,
		stream: stream,
	}
}

func (m *Match) Info() MatchInfo {
	return m.info
}

func (m *Match) SendMove(ctx context.Context, move Move) error {
	whitePeerBytes, _ := m.info.White.Marshal()
	blackPeerBytes, _ := m.info.Black.Marshal()

	moveSigBytes := bytes.NewBuffer(nil)
	moveSigBytes.Write(m.info.ID[:])
	moveSigBytes.Write(whitePeerBytes)
	moveSigBytes.Write(blackPeerBytes)
	if err := binary.Write(moveSigBytes, binary.BigEndian, move); err != nil {
		return err
	}
	if err := binary.Write(moveSigBytes, binary.BigEndian, move.SeqNum); err != nil {
		return err
	}

	sig, err := m.stream.Conn().LocalPrivateKey().Sign(moveSigBytes.Bytes())
	if err != nil {
		return err
	}

	enc := uint32(move.FromRow) | (uint32(move.FromCol) << 8) | (uint32(move.ToRow) << 16) | (uint32(move.ToCol) << 24)

	m.logger.Debug("sending signed move")
	msg := &ipchessproto.Move{
		MatchId:   m.info.ID[:],
		White:     whitePeerBytes,
		Black:     blackPeerBytes,
		Enc:       enc,
		Seq:       move.SeqNum,
		Signature: sig,
	}
	if err := sendMessage(ctx, m.stream, msg); err != nil {
		return err
	}

	return nil
}

func (m *Match) ReceiveMove(ctx context.Context) (Move, error) {
	m.logger.Debug("waiting signed move")
	var moveMsg ipchessproto.Move
	if err := receiveMessage(ctx, m.stream, &moveMsg); err != nil {
		return Move{}, err
	}

	var dec Move
	dec.FromRow = uint8(moveMsg.Enc & 0xff)
	dec.FromCol = uint8((moveMsg.Enc >> 8) & 0xff)
	dec.ToRow = uint8((moveMsg.Enc >> 16) & 0xff)
	dec.ToCol = uint8((moveMsg.Enc >> 24) & 0xff)
	dec.SeqNum = moveMsg.Seq

	signedMoveBytes := bytes.NewBuffer(nil)
	signedMoveBytes.Write(moveMsg.MatchId)
	signedMoveBytes.Write(moveMsg.White)
	signedMoveBytes.Write(moveMsg.Black)
	if err := binary.Write(signedMoveBytes, binary.BigEndian, dec); err != nil {
		return Move{}, err
	}
	if err := binary.Write(signedMoveBytes, binary.BigEndian, moveMsg.Seq); err != nil {
		return Move{}, err
	}

	m.logger.Debug("verifying move signature")
	valid, err := m.stream.Conn().RemotePublicKey().Verify(signedMoveBytes.Bytes(), moveMsg.Signature)
	if err != nil {
		return Move{}, err
	}
	if !valid {
		return Move{}, errInvalidMoveSignature
	}

	return dec, nil
}

func (m *Match) Close() {
	m.stream.Close()
}
