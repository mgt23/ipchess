import { app, BrowserWindow, Menu } from "electron";
import { MenuItemConstructorOptions } from "electron/main";
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
    title: "InterPlanetary Chess",
    show: false,
    backgroundColor: "#21121f", // TODO: get this from tailwind config?
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
  });

  const menuTemplate: MenuItemConstructorOptions[] = [
    {
      role: "help",
      submenu: [
        {
          label: "Toggle developer tools",
          accelerator: "F12",
          click: () => {
            if (window.webContents.isDevToolsOpened()) {
              window.webContents.closeDevTools();
            } else {
              window.webContents.openDevTools();
            }
          },
        },
      ],
    },
  ];

  if (process.platform === "darwin") {
    menuTemplate.unshift({
      role: "appMenu",
      label: "InterPlantary Chess",
    });
  }

  window.setMenu(Menu.buildFromTemplate(menuTemplate));
  window.setMenuBarVisibility(false);

  window.once("ready-to-show", () => {
    log.debug("showing window");
    window.show();
  });

  const appStateStart = appState.start();

  await window.loadURL("http://localhost:1234");

  await appStateStart;

  if (appState.jsonrpcClient) {
    const nodeId = await appState.jsonrpcClient.call("node_id");
    window.webContents.send("app.initialized", { nodeId });
  }
}

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

main();
