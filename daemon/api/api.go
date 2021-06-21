package api

import (
	"context"
	"fmt"
	"ipchess/p2p"
	"net/http"

	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

type Option func(*Handler)

func WithLogger(logger *zap.Logger) Option {
	return func(handler *Handler) {
		handler.logger = logger
	}
}

var wsUpgrader websocket.Upgrader

// Handler handles API RPC requests.
type Handler struct {
	ctx       context.Context
	ctxCancel context.CancelFunc

	logger *zap.Logger
	host   *p2p.Host
	server http.Server
}

// NewHandler creates a new API handler.
func NewHandler(host *p2p.Host, options ...Option) *Handler {
	ctx, ctxCancel := context.WithCancel(context.Background())
	h := &Handler{
		ctx:       ctx,
		ctxCancel: ctxCancel,
		logger:    zap.NewNop(),
		host:      host,
	}

	for _, option := range options {
		option(h)
	}

	return h
}

// ServeHTTP implements handling incoming websocket connections.
func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := wsUpgrader.Upgrade(w, r, nil)
	if err != nil {
		h.logger.Error("failed upgrading HTTP connection", zap.Error(err))
		return
	}

	h.logger.Debug("handling new Websocket API connection")
	go h.handleConn(conn)
}

// StartLocal starts the API listening only to 127.0.0.1 at the given port.
func (h *Handler) StartLocal(port uint16) {
	h.server.Addr = fmt.Sprintf("127.0.0.1:%d", port)
	h.server.Handler = h

	go func() {
		err := h.server.ListenAndServe()
		if err != nil && err != http.ErrServerClosed {
			h.logger.Error("API server error", zap.Error(err))
		}
	}()
}

// Shutdown blocks until the underlying HTTP server has gracefully shutdown.
func (h *Handler) Shutdown() {
	h.ctxCancel()
	h.server.Shutdown(context.Background())
}

func (h *Handler) handleConn(conn *websocket.Conn) {
	defer conn.Close()

	responseHandlerCtx, cancelResponseHandlerCtx := context.WithCancel(h.ctx)
	defer cancelResponseHandlerCtx()
	responseChan := make(chan *jsonRPCResponse)
	go func() {
		for {
			select {
			case <-responseHandlerCtx.Done():
				return
			case response := <-responseChan:
				err := writeJSONRPCResponse(conn, *response)
				if err != nil {
					h.logger.Debug("failed sending JSONRPC response", zap.Error(err))
				}
			}
		}
	}()

	for {
		request, requestID, err := readJSONRPCRequest(conn)
		if _, isCloseError := err.(*websocket.CloseError); isCloseError {
			h.logger.Debug("Websocket connection closed")
			break
		}

		if err == errInvalidParamsType {
			responseChan <- &jsonRPCResponse{
				Error: invalidRequestError(),
				ID:    requestID,
			}
		} else if err != nil {
			h.logger.Debug("failed reading JSONRPC message", zap.Error(err))
			responseChan <- &jsonRPCResponse{
				Error: &jsonRPCResponseError{
					Code:    -32700,
					Message: "Parse error",
				},
			}
		} else {
			switch request.Method {
			case "node_id":
				responseChan <- &jsonRPCResponse{
					Result: h.host.ID().Pretty(),
					ID:     request.ID,
				}

			default:
				responseChan <- &jsonRPCResponse{
					Error: &jsonRPCResponseError{
						Code:    -32601,
						Message: "Method not found",
					},
					ID: requestID,
				}
			}
		}
	}
}
