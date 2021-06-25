import { BoardData, PieceColor } from "../lib";
import { RouterPage } from "./router";

export type AppState = {
  initializing: boolean;

  node: {
    id: string | null;
  };

  router: {
    currentPage: RouterPage;
  };

  match?: {
    opponent: {
      id: string;
    };
    playerPieceColor: PieceColor;
    selection?: { row: number; column: number };
    boardData: BoardData;
  };
};

export const initialAppState = (): AppState => ({
  initializing: true,

  node: {
    id: null,
  },

  router: {
    currentPage: "splash",
  },

  match: {
    opponent: {
      id: "somerandompeerid",
    },
    playerPieceColor: "black",
    boardData: new BoardData(),
  },
});

export type AppMessage =
  | {
      type: "initialization-finished";
      payload: {
        nodeId: string;
      };
    }
  | {
      type: "challenge-accepted";
    }
  | {
      type: "piece-selected";
      payload: {
        row: number;
        column: number;
      };
    }
  | {
      type: "selected-piece-moved";
      payload: {
        toRow: number;
        toColumn: number;
      };
    };

export const update = (state: AppState, message: AppMessage): AppState => {
  switch (message.type) {
    case "initialization-finished":
      return {
        ...state,
        initializing: false,
        node: {
          ...state.node,
          id: message.payload.nodeId,
        },
        router: { ...state.router, currentPage: "home" },
      };

    case "challenge-accepted":
      return {
        ...state,
        router: { ...state.router, currentPage: "match" },
      };

    case "piece-selected":
      return {
        ...state,
        match: { ...state.match, selection: message.payload },
      };

    case "selected-piece-moved": {
      console.log(
        `moved from ${JSON.stringify(
          state.match.selection
        )} to ${JSON.stringify(message.payload)}`
      );
      return {
        ...state,
        match: {
          ...state.match,
          selection: null,
        },
      };
    }
  }
};
