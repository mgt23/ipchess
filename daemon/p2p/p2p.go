package p2p

import (
	"github.com/ipfs/go-cid"
	"github.com/multiformats/go-multihash"
)

const ipchessProtocolID = "ipchess/0.1.0"

var cidPrefix = cid.Prefix{
	Version:  1,
	Codec:    cid.Raw,
	MhType:   multihash.SHA2_256,
	MhLength: -1,
}
