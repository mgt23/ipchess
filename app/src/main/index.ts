import { app, BrowserWindow } from "electron";

app.whenReady().then(() => {
    const window = new BrowserWindow({
        width: 800,
        height: 600,
    });

    // dev url for parcel
    window.loadURL("http://localhost:1234");
});

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        app.quit();
    }
});