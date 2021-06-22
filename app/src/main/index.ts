import { app, BrowserWindow } from "electron";
import log, { LoggingMethod } from "loglevel";
import { State } from "./state";

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

const appState = new State();

async function main() {
    await app.whenReady();
    appState.start();

    let quitTryCount = 0;
    app.on("before-quit", (e) => {
        if (quitTryCount >= 200) {
            appState.terminate();
            return;
        }

        if (!appState.isClosed()) {
            e.preventDefault();
            appState.close();
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
