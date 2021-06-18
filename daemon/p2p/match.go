package p2p

import "github.com/libp2p/go-libp2p-core/peer"

// Match holds data about a InterPlanetary Chess match.
type Match struct {
	ID    [32]byte
	White peer.ID
	Black peer.ID
}
