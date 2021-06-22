import childProcess, { ChildProcessWithoutNullStreams } from "child_process";
import WebSocket from "ws";
import getPort from "get-port";
import log from "loglevel";
import path from "path";
import { app } from "electron";

export class State {
    private daemonProcess: ChildProcessWithoutNullStreams | null;
    private wsConnection: WebSocket | null;

    private daemonApiPort: number;

    constructor() {
        this.daemonProcess = null;
        this.wsConnection = null;
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
            this.wsConnection === null ||
            this.wsConnection.readyState === WebSocket.CLOSED;

        return daemonProcessClosed && wsConnectionClosed;
    }

    close() {
        if (
            this.wsConnection &&
            this.wsConnection.readyState !== WebSocket.CLOSED &&
            this.wsConnection.readyState !== WebSocket.CLOSING
        ) {
            this.wsConnection.close();
        }

        if (this.daemonProcess && this.daemonProcess.pid) {
            process.kill(this.daemonProcess.pid);
        }
    }

    terminate() {
        if (
            this.wsConnection &&
            this.wsConnection.readyState !== WebSocket.CLOSED
        ) {
            this.wsConnection.terminate();
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

        log.info(
            `starting local daemon process API_PORT=${this.daemonApiPort}`
        );
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
                const ws = new WebSocket(
                    `ws://127.0.0.1:${this.daemonApiPort}`
                );

                ws.on("open", () => {
                    log.debug("daemon process WebSocket connection opened");

                    this.wsConnection = ws;
                    this.wsConnection.removeAllListeners();
                    this.wsConnection.on("message", (data: WebSocket.Data) => {
                        log.debug(
                            `received data from daemon process DATA=${data.toString()}`
                        );
                    });
                    this.wsConnection.on("error", (err) => {
                        log.error(err);
                    });
                    this.wsConnection.on("close", () => {
                        log.debug("daemon process WebSocket connection closed");
                        this.wsConnection = null;
                    });

                    this.wsConnection.send(
                        JSON.stringify({
                            jsonrpc: "2.0",
                            method: "node_id",
                            id: 1,
                        })
                    );

                    resolve(true);
                });

                ws.on("close", () => {
                    resolve(false);
                });
                ws.on("error", () => {
                    resolve(false);
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
