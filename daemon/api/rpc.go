package api

import (
	"encoding/json"
	"errors"

	"github.com/gorilla/websocket"
)

var (
	errInvalidJSONRPCVersion = errors.New("jsonrpc field should be exactly equal to \"2.0\"")
	errInvalidParamsType     = errors.New("params field should be object or array")
)

// jsonRPCRequest represents the JSONRPC request format.
type jsonRPCRequest struct {
	JSONRPC string       `json:"jsonrpc"`
	Method  string       `json:"method"`
	Params  interface{}  `json:"params"`
	ID      *json.Number `json:"id"`
}

// readJSONRPCRequest reads a valid JSONRPC request from a Websocket stream.
func readJSONRPCRequest(conn *websocket.Conn) (jsonRPCRequest, *json.Number, error) {
	var msg jsonRPCRequest
	if err := conn.ReadJSON(&msg); err != nil {
		return jsonRPCRequest{}, nil, err
	}

	if msg.JSONRPC != "2.0" {
		return jsonRPCRequest{}, msg.ID, errInvalidJSONRPCVersion
	}

	switch msg.Params.(type) {
	case nil, map[string]interface{}, []interface{}:
	default:
		return jsonRPCRequest{}, msg.ID, errInvalidParamsType
	}

	return msg, msg.ID, nil
}

// jsonRPCResponseError represents the JSONRPC response error field format.
type jsonRPCResponseError struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    interface{} `json:"data,omitempty"`
}

// jsonRPCResponseError represents the JSONRPC response format.
type jsonRPCResponse struct {
	JSONRPC string                `json:"jsonrpc"`
	Result  interface{}           `json:"result,omitempty"`
	Error   *jsonRPCResponseError `json:"error,omitempty"`
	ID      *json.Number          `json:"id,omitempty"`
}

// invalidRequestError constructs a new JSONRPC invalid request error.
func invalidRequestError() *jsonRPCResponseError {
	return &jsonRPCResponseError{
		Code:    -32600,
		Message: "Invalid Request",
	}
}

// readJSONRPCRequest writes a JSONRPC response to a Websocket stream.
func writeJSONRPCResponse(conn *websocket.Conn, response jsonRPCResponse) error {
	response.JSONRPC = "2.0"
	return conn.WriteJSON(response)
}
