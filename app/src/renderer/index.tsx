import { ipcRenderer } from "electron";
import React, { useEffect, useReducer } from "react";
import { render } from "react-dom";

import router from "./router";
import { AppMessage, initialAppState, update } from "./state";

const App = () => {
  const [state, rawDispatch] = useReducer(update, initialAppState());

  const dispatch = (msg: AppMessage) => {
    console.log(`dispatching ${JSON.stringify(msg)}`);
    rawDispatch(msg);
  };

  useEffect(() => {
    ipcRenderer.on("app.initialized", (_event, { nodeId }) => {
      dispatch({ type: "initialization-finished", payload: { nodeId } });
    });

    ipcRenderer.on("challenge.received", (_event, { peerId }) => {
      dispatch({ type: "received-peer-challenge", payload: { peerId } });
    });

    ipcRenderer.on("match.ready", (_event, { peerId }) => {
      dispatch({ type: "match-ready", payload: { peerId } });
    });

    return () => {
      ipcRenderer.removeAllListeners("app.initialized");
      ipcRenderer.removeAllListeners("challenge.received");
      ipcRenderer.removeAllListeners("match.ready");
    };
  });

  return <div>{router(state, dispatch)}</div>;
};

render(<App />, document.getElementById("react-root"));
