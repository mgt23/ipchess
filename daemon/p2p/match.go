package p2p

import (
	"github.com/google/uuid"
	"github.com/ipfs/go-cid"
)

type Match struct {
	CID cid.Cid
	ID  uuid.UUID
}

func newMatch() *Match {
	matchID := uuid.New()
	matchCID, _ := cidPrefix.Sum(matchID[:])

	return &Match{
		CID: matchCID,
		ID:  matchID,
	}
}
