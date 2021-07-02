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

  sentChallenges: { [key: string]: Challenge };
  receivedChallenges: { [key: string]: Challenge };

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

  sentChallenges: {},
  receivedChallenges: {},

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
      type: "challenged-peer";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "challenge-canceled";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "challenge-declined";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "received-challenge";
      payload: {
        peerId: string;
      };
    }
  | {
      type: "peer-canceled-challenge";
      payload: { peerId: string };
    }
  | {
      type: "peer-declined-challenge";
      payload: { peerId: string };
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
  const nextState = ((): AppState => {
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

      case "challenged-peer":
        return {
          ...state,
          sentChallenges: {
            ...state.sentChallenges,
            [message.payload.peerId]: {
              peerId: message.payload.peerId,
            },
          },
        };

      case "challenge-canceled":
      case "peer-declined-challenge": {
        const nextState = { ...state };
        delete nextState.sentChallenges[message.payload.peerId];

        return nextState;
      }

      case "challenge-declined":
      case "peer-canceled-challenge": {
        const nextState = { ...state };
        delete nextState.receivedChallenges[message.payload.peerId];

        return nextState;
      }

      case "received-challenge":
        return {
          ...state,
          receivedChallenges: {
            ...state.receivedChallenges,
            [message.payload.peerId]: {
              peerId: message.payload.peerId,
            },
          },
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
        return {
          ...state,
          match: {
            ...state.match,
            selection: null,
          },
        };
      }
    }
  })();

  console.log(`state updated: ${JSON.stringify(nextState)}`);
  return nextState;
};
