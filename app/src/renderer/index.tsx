import { ipcRenderer } from "electron";
import React, { useEffect, useReducer } from "react";
import { render } from "react-dom";

import router from "./router";
import { initialAppState, update } from "./state";

const App = () => {
  const [state, dispatch] = useReducer(update, initialAppState());

  useEffect(() => {
    ipcRenderer.on("app.initialized", (_event, data) => {
      const { nodeId } = data;
      dispatch({ type: "initialization-finished", payload: { nodeId } });
    });

    return () => {
      ipcRenderer.removeAllListeners("app.initialized");
    };
  });

  return <div>{router(state, dispatch)}</div>;
};

render(<App />, document.getElementById("react-root"));
