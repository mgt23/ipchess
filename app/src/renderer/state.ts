import { BoardData, PieceColor } from "../lib";
import { RouterPage } from "./router";

export type Challenge = {
  peerId: string;
};

export type AppState = {
  initializing: boolean;

  node: {
    id: string | null;
  };

  router: {
    currentPage: RouterPage;
  };

  sentChallenge: Challenge | null;
  receivedChallenges: Array<Challenge>;

  match: {
    opponent: {
      id: string;
    };
    playerPieceColor: PieceColor;
    selection?: { row: number; column: number };
    boardData: BoardData;
  } | null;
};

export const initialAppState = (): AppState => ({
  initializing: true,

  node: {
    id: null,
  },

  sentChallenge: null,
  receivedChallenges: [],

  router: {
    currentPage: "splash",
  },

  match: null,
});

export type AppMessage =
  | {
      type: "initialization-finished";
      payload: {
        nodeId: string;
      };
    }
  | {
      type: "peer-challenged";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "received-peer-challenge";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "match-ready";
      payload: {
        peerId: string;
      };
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

    case "peer-challenged":
      return {
        ...state,
        sentChallenge: {
          peerId: message.payload.peerId,
        },
      };

    case "received-peer-challenge":
      return {
        ...state,
        receivedChallenges: [
          ...state.receivedChallenges,
          {
            peerId: message.payload.peerId,
          },
        ],
      };

    case "match-ready":
      return {
        ...state,
        router: { ...state.router, currentPage: "match" },
        match: {
          opponent: {
            id: message.payload.peerId,
          },
          playerPieceColor: "white",
          boardData: new BoardData(),
        },
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
