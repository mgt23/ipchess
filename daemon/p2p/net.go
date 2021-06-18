package p2p

import (
	"context"
	"encoding/binary"
	"errors"

	"github.com/libp2p/go-libp2p-core/network"
	"google.golang.org/protobuf/proto"
)

const maxMessageLength = 1024

var (
	errMaxMessageLengthExceeded = errors.New("max message length exceeded")
)

// sendMessage tries to send a length-prefixed protobuf encoded message.
func sendMessage(ctx context.Context, stream network.Stream, message proto.Message) error {
	msgBytes, err := proto.Marshal(message)
	if err != nil {
		return err
	}

	msgLength := uint16(len(msgBytes))
	if msgLength > maxMessageLength {
		return errMaxMessageLengthExceeded
	}

	ctxDeadline, _ := ctx.Deadline()
	if err := stream.SetWriteDeadline(ctxDeadline); err != nil {
		return err
	}

	if err := binary.Write(stream, binary.BigEndian, msgLength); err != nil {
		return err
	}
	if _, err := stream.Write(msgBytes); err != nil {
		return err
	}

	return nil
}

// receiveMessage tries to receive a length-prefixed protobuf encoded message.
func receiveMessage(ctx context.Context, stream network.Stream, message proto.Message) error {
	ctxDeadline, _ := ctx.Deadline()
	if err := stream.SetReadDeadline(ctxDeadline); err != nil {
		return err
	}

	var msgLength uint16
	if err := binary.Read(stream, binary.BigEndian, &msgLength); err != nil {
		return err
	}

	if msgLength > maxMessageLength {
		return errMaxMessageLengthExceeded
	}

	msgBytes := make([]byte, msgLength)
	if _, err := stream.Read(msgBytes); err != nil {
		return err
	}

	if err := proto.Unmarshal(msgBytes, message); err != nil {
		return err
	}

	return nil
}
