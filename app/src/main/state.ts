import childProcess, { ChildProcessWithoutNullStreams } from "child_process";
import { app } from "electron";
import getPort from "get-port";
import log from "loglevel";
import path from "path";
import WebSocket from "ws";
import * as jsonrpc from "./jsonrpc";

export class State {
  daemonProcess: ChildProcessWithoutNullStreams | null;
  jsonrpcClient: jsonrpc.Client | null;

  daemonApiPort: number;

  constructor() {
    this.daemonProcess = null;
    this.jsonrpcClient = null;
    this.daemonApiPort = 0;
  }

  async start() {
    await this.startDaemonProcess();
    await this.startDaemonWebSocketConnection();
  }

  isClosed(): boolean {
    const daemonProcessClosed =
      this.daemonProcess === null || this.daemonProcess.pid === null;
    const wsConnectionClosed =
      this.jsonrpcClient === null || this.jsonrpcClient.closed();

    return daemonProcessClosed && wsConnectionClosed;
  }

  close() {
    if (this.jsonrpcClient) {
      this.jsonrpcClient.close();
    }

    if (this.daemonProcess && this.daemonProcess.pid) {
      process.kill(this.daemonProcess.pid);
    }
  }

  terminate() {
    if (this.jsonrpcClient && !this.jsonrpcClient.closed()) {
      this.jsonrpcClient.terminate();
    }

    if (this.daemonProcess && this.daemonProcess.pid) {
      process.kill(this.daemonProcess.pid, "SIGKILL");
    }
  }

  private async startDaemonProcess() {
    this.daemonApiPort = await getPort({
      port: 3030,
      host: "127.0.0.1",
    });

    log.info(`starting local daemon process API_PORT=${this.daemonApiPort}`);
    const daemonProcess = childProcess.spawn(
      path.join(app.getAppPath(), "ipchess"),
      ["daemon", "--api.port", this.daemonApiPort.toString()],
      { detached: false, killSignal: "SIGTERM" }
    );
    log.debug(`daemon processes started PID=${daemonProcess.pid}`);

    daemonProcess.on("close", () => {
      log.info("daemon process closed");
      this.daemonProcess = null;
    });

    this.daemonProcess = daemonProcess;
  }

  private async startDaemonWebSocketConnection() {
    log.debug(`connecting to daemon API API_PORT=${this.daemonApiPort}`);

    const wsTryConnect = () =>
      new Promise<boolean>((resolve) => {
        const ws = new WebSocket(`ws://127.0.0.1:${this.daemonApiPort}`);

        ws.on("close", () => {
          resolve(false);
        });

        ws.on("error", () => {
          resolve(false);
        });

        ws.on("open", () => {
          log.debug("daemon process WebSocket connection opened");

          ws.removeAllListeners();
          ws.on("error", (err) => {
            log.error(err);
          });

          ws.on("close", () => {
            log.debug("daemon process WebSocket connection closed");
            this.jsonrpcClient = null;
          });

          this.jsonrpcClient = new jsonrpc.Client(ws);

          resolve(true);
        });
      });

    // keep trying until the daemon's process responds
    for (;;) {
      if (await wsTryConnect()) {
        break;
      }

      await new Promise<void>((resolve) => setTimeout(resolve, 10));
    }
  }
}
