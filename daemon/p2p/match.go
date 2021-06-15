package p2p

import (
	"github.com/google/uuid"
	"github.com/ipfs/go-cid"
)

type Match struct {
	contentID cid.Cid
	id        uuid.UUID
}
