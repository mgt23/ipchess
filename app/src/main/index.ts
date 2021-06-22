import childProcess from "child_process";
import { ChildProcessWithoutNullStreams } from "child_process";
import path from "path";
import { app, BrowserWindow } from "electron";

let daemonProcess: ChildProcessWithoutNullStreams | null = null;

app.whenReady().then(() => {
    const window = new BrowserWindow({
        width: 800,
        height: 600,
    });

    daemonProcess = childProcess.spawn(
        path.join(app.getAppPath(), "ipchess"),
        ["daemon", "--api.port", "3030"],
        { detached: false, killSignal: "SIGTERM" }
    );
    daemonProcess.stdout.on("data", (chunk: Buffer) => {
        console.log(chunk.toString("utf-8"));
    });
    daemonProcess.stderr.on("data", (chunk: Buffer) => {
        console.log(chunk.toString("utf-8"));
    });
    daemonProcess.on("close", () => {
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

    // dev url for parcel
    window.loadURL("http://localhost:1234");
});

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        app.quit();
    }
});
