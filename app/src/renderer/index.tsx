import { ipcRenderer } from "electron";
import React, { useEffect, useReducer } from "react";
import { render } from "react-dom";

import router from "./router";
import { AppMessage, initialAppState, update } from "./state";

type IpcListener = (event: Electron.IpcRendererEvent, ...args: any[]) => void;
type IpcListenersFunc = (dispatch: React.Dispatch<AppMessage>) => {
  [key: string]: IpcListener;
};

const ipcListenersFunc: IpcListenersFunc = (dispatch) => ({
  "app.initialized": (_event, { nodeId }) => {
    dispatch({ type: "initialization-finished", payload: { nodeId } });
  },

  "challenge.received": (_event, { peerId }) => {
    dispatch({ type: "received-challenge", payload: { peerId } });
  },

  "challenge.send": (_event, { peerId }) => {
    dispatch({ type: "challenged-peer", payload: { peerId } });
  },

  "challenge.cancel": (_event, { peerId }) => {
    dispatch({ type: "challenge-canceled", payload: { peerId } });
  },

  "challenge.decline": (_event, { peerId }) => {
    dispatch({ type: "challenge-declined", payload: { peerId } });
  },

  "challenge.accept": (_event, { peerId }) => {},

  "challenge.peer-canceled": (_event, { peerId }) => {
    dispatch({ type: "peer-canceled-challenge", payload: { peerId } });
  },

  "challenge.peer-declined": (_event, { peerId }) => {
    dispatch({ type: "peer-declined-challenge", payload: { peerId } });
  },

  "challenge.peer-accepted": (_event, { peerId }) => {
    dispatch({ type: "match-ready", payload: { peerId } });
  },
});

const App = () => {
  const [state, rawDispatch] = useReducer(update, initialAppState());

  const dispatch = (msg: AppMessage) => {
    console.log(`dispatching ${JSON.stringify(msg)}`);
    rawDispatch(msg);
  };

  useEffect(() => {
    const ipcListeners = ipcListenersFunc(dispatch);

    Object.entries(ipcListeners).forEach(([channel, listener]) =>
      ipcRenderer.on(channel, listener)
    );

    return () => {
      Object.keys(ipcListeners).forEach((channel) =>
        ipcRenderer.removeAllListeners(channel)
      );
    };
  });

  return <div>{router(state, dispatch)}</div>;
};

render(<App />, document.getElementById("react-root"));
