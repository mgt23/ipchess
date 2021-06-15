package p2p

import (
	"github.com/google/uuid"
	"github.com/ipfs/go-cid"
	"github.com/libp2p/go-libp2p-core/peer"
)

type Match struct {
	CID      cid.Cid
	ID       uuid.UUID
	Provider peer.ID
}

func newMatch(provider peer.ID) *Match {
	matchID := uuid.New()
	matchCID, _ := cidPrefix.Sum(matchID[:])

	return &Match{
		CID:      matchCID,
		ID:       matchID,
		Provider: provider,
	}
}
