import { app, BrowserWindow, ipcMain, Menu } from "electron";
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
    appState.jsonrpcClient.addNotificationListener(
      (subscriptionId, method, data) => {
        if (method !== "subscribe_events") {
          log.warn(
            `ignoring daemon notification METHOD=${method} DATA=${JSON.stringify(
              data
            )}`
          );
          return;
        }

        log.debug(
          `received notification SUBSCRIPTION=${subscriptionId} METHOD=${method} DATA=${JSON.stringify(
            data
          )}`
        );

        const { event_type: eventType, data: eventData } = data;

        switch (eventType) {
          case "peer_challenge":
            {
              const { peer_id: peerId } = eventData;
              window.webContents.send("challenge.received", { peerId });
            }
            break;

          case "challenge_canceled":
            {
              const { peer_id: peerId } = eventData;
              window.webContents.send("challenge.peer-canceled", { peerId });
            }
            break;

          case "challenge_declined":
            {
              const { peer_id: peerId } = eventData;
              window.webContents.send("challenge.peer-declined", { peerId });
            }
            break;

          case "challenge_accepted":
            {
              const { peer_id: peerId } = eventData;
              window.webContents.send("challenge.peer-accepted", { peerId });
            }
            break;
        }
      }
    );

    for (;;) {
      const isConnected = await appState.jsonrpcClient.call("is_connected");

      if (isConnected) {
        break;
      }

      await new Promise((resolve) => setTimeout(resolve, 100));
    }

    await appState.jsonrpcClient.call("subscribe_events");
    const nodeId = await appState.jsonrpcClient.call("node_id");
    window.webContents.send("app.initialized", { nodeId });
  }

  ipcMain.on("challenge.send", async (event, peerId) => {
    if (appState.jsonrpcClient === null) {
      return;
    }

    appState.jsonrpcClient.call("challenge_peer", [peerId]).then(() => {
      event.reply("challenge.send", { peerId });
    });
  });

  ipcMain.on("challenge.cancel", async (event, peerId) => {
    if (appState.jsonrpcClient === null) {
      return;
    }

    log.debug(`cancelling peer challenge ${peerId}`);
    appState.jsonrpcClient.call("cancel_challenge", [peerId]).then(() => {
      event.reply("challenge.cancel", { peerId });
    });
  });

  ipcMain.on("challenge.decline", async (event, peerId) => {
    if (appState.jsonrpcClient === null) {
      return;
    }

    log.debug(`declining peer challenge ${peerId}`);
    appState.jsonrpcClient.call("decline_peer_challenge", [peerId]).then(() => {
      event.reply("challenge.decline", { peerId });
    });
  });

  ipcMain.on("challenge.accept", async (event, peerId) => {
    if (appState.jsonrpcClient === null) {
      return;
    }

    log.debug(`accepting peer challenge ${peerId}`);
    appState.jsonrpcClient.call("accept_peer_challenge", [peerId]).then(() => {
      event.reply("challenge.accept", { peerId });
    });
  });
}

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

main();
