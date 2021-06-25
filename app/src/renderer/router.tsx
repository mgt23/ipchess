import React from "react";

import HomePage from "./pages/Home";
import SpashPage from "./pages/Splash";
import MatchPage from "./pages/Match";
import { AppMessage, AppState } from "./state";

type PageFunc = (
  state: AppState,
  dispatch: React.Dispatch<AppMessage>
) => React.ReactElement;

export type RouterPage = "splash" | "home" | "match";

const pages: { [key in RouterPage]: PageFunc } = {
  splash: () => <SpashPage />,
  home: (state, dispatch) => <HomePage state={state} dispatch={dispatch} />,
  match: (state, dispatch) => <MatchPage state={state} dispatch={dispatch} />,
};

export default (state: AppState, dispatch: React.Dispatch<AppMessage>) => {
  return pages[state.router.currentPage](state, dispatch);
};
