import log from "loglevel";
import WebSocket from "ws";

export type Error = {
  code: number;
  message: string;
  data?: any;
};

type ClientPendingRequests = { [key in string | number]: (value: any) => void };

export class Client {
  private conn: WebSocket;
  private pendingRequests: ClientPendingRequests;
  private requestIdCounter: number;

  constructor(conn: WebSocket) {
    this.conn = conn;
    this.pendingRequests = {};
    this.requestIdCounter = 1;

    this.conn.on("message", (data) => {
      log.debug(`message from daemon process DATA=${data.toString("utf-8")}`);
      const parsedMessage = JSON.parse(data.toString("utf-8"));

      if (parsedMessage.jsonrpc !== "2.0") {
        log.error(
          "invalid message from daemon process REASON=invalid jsonrpc field"
        );
        return;
      }

      const err = parsedMessage.error;
      if (
        err &&
        (typeof err.code !== "number" || typeof err.message !== "string")
      ) {
        log.error(
          "invalid message from daemon process REASON=invalid error struct"
        );
        return;
      }

      const id = parsedMessage.id;
      if (id && (typeof id === "string" || typeof id === "number")) {
        const resolve = this.pendingRequests[id];

        if (resolve) {
          delete this.pendingRequests[id];

          if (parsedMessage.result !== undefined) {
            resolve(parsedMessage.result);
          } else if (parsedMessage.error !== undefined) {
            resolve(Promise.reject(parsedMessage.error as Error));
          }
        }
      }
    });
  }

  terminate() {
    this.conn.terminate();
  }

  close() {
    if (
      this.conn.readyState !== WebSocket.CLOSED &&
      this.conn.readyState !== WebSocket.CLOSING
    ) {
      this.conn.close();
    }
  }

  closed(): boolean {
    return this.conn.readyState === WebSocket.CLOSED;
  }

  call(method: string, params?: Array<any> | Object): Promise<any> {
    const request = {
      jsonrpc: "2.0",
      id: this.requestIdCounter++,
      method,
      params,
    };

    this.conn.send(JSON.stringify(request));

    return new Promise((resolve) => {
      if (request.id) {
        this.pendingRequests[request.id] = resolve;
      }
    });
  }
}
