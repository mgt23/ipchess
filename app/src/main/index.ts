import { app, BrowserWindow } from "electron";
import childProcess, { ChildProcessWithoutNullStreams } from "child_process";
import getPort from "get-port";
import log, { LoggingMethod } from "loglevel";
import path from "path";

const originalFactory = log.methodFactory;
log.methodFactory = (methodName, logLevel, loggerName): LoggingMethod => {
    return (message) => {
        return originalFactory(
            methodName,
            logLevel,
            loggerName
        )(`${new Date().toISOString()} ${methodName.toUpperCase()} ${message}`);
    };
};
log.setDefaultLevel(log.levels.DEBUG);
log.setLevel(log.getLevel());

async function main() {
    await app.whenReady();

    let daemonProcess: ChildProcessWithoutNullStreams | null = null;
    const daemonApiPort = await getPort({
        port: 3030,
        host: "127.0.0.1",
    });

    log.info(`starting daemon process with API at 127.0.0.1:${daemonApiPort}`);
    daemonProcess = childProcess.spawn(
        path.join(app.getAppPath(), "ipchess"),
        ["daemon", "--api.port", daemonApiPort.toString()],
        { detached: false, killSignal: "SIGTERM" }
    );
    daemonProcess.on("spawn", () => {
        log.info("daemon process started");
    });
    daemonProcess.on("close", () => {
        log.info("daemon process closed");
        daemonProcess = null;
    });

    app.on("before-quit", (e) => {
        if (daemonProcess && daemonProcess.pid) {
            e.preventDefault();
            process.kill(daemonProcess.pid);
            // try again after a while
            setTimeout(() => app.quit(), 10);
        }
    });

    const window = new BrowserWindow({
        width: 800,
        height: 600,
    });

    // dev url for parcel
    await window.loadURL("http://localhost:1234");
}

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        app.quit();
    }
});

main();
