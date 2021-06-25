import React from "react";
import Board from "../components/Board";
import { AppMessage, AppState } from "../state";

export type MatchPageProps = {
  state: AppState;
  dispatch: React.Dispatch<AppMessage>;
};

const MatchPage: React.FunctionComponent<MatchPageProps> = ({
  state,
  dispatch,
}: MatchPageProps) => {
  if (state.match === null) {
    return <div>Not in a match</div>;
  } else {
    const { match } = state;

    return (
      <div className="bg-primary text-white flex flex-row justify-center h-screen">
        <div className="flex flex-col justify-center p-2 w-full h-full md:max-w-screen-sm lg:max-w-screen-md md:space-y-4 lg:space-y-8">
          <div>
            <div className="md:text-lg lg:text-xl font-bold">Opponent's ID</div>
            <div className="md:text-md lg:text-lg">{match.opponent.id}</div>
          </div>

          <div className="w-full overflow-hidden">
            <Board
              data={match.boardData}
              playerPieceColor={match.playerPieceColor}
              selection={match.selection}
              onTileClick={(row, column, piece) => {
                if (piece && piece.color === match.playerPieceColor) {
                  dispatch({
                    type: "piece-selected",
                    payload: { row, column },
                  });
                } else if (match.selection) {
                  // should check with daemon if the attempted move is
                  // valid and then, iff it is, dispatch this message
                  dispatch({
                    type: "selected-piece-moved",
                    payload: {
                      toRow: row,
                      toColumn: column,
                    },
                  });
                }
              }}
            />
          </div>
        </div>

        <div className="flex flex-col justify-center space-y-2 w-full h-full md:p-2 lg:p-3 md:max-w-xs lg:max-w-sm">
          <div className="bg-primary-light h-full p-2 rounded font-bold md:text-lg lg:text-xl">
            Moves
          </div>
          <div className="bg-light-light h-full p-2 rounded flex flex-col">
            <div className="text-primary font-bold md:text-lg lg:text-xl">
              Chat
            </div>

            <div className="w-full h-full">{/* chat content */}</div>

            <div className="flex flex-row w-full items-start space-x-1">
              <input className="text-primary shadow rounded w-full h-8 pl-1 pr-1 focus:outline-none"></input>
              <div className="flex justify-center items-center bg-primary hover:bg-primary-light focus:outline-none text-white shadow-md font-bold rounded h-8 pl-1 pr-1 select-none cursor-pointer">
                Send
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }
};

export default MatchPage;
