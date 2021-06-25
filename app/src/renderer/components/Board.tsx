import React from "react";
import { BoardData, Piece, PieceColor, PieceKind } from "../../lib";
import {
  BlackBishop,
  BlackKing,
  BlackKnight,
  BlackPawn,
  BlackQueen,
  BlackRook,
  PieceProps,
  WhiteBishop,
  WhiteKing,
  WhiteKnight,
  WhitePawn,
  WhiteQueen,
  WhiteRook,
} from "./pieces";

const pieceMap: {
  [key in PieceColor]: {
    [key in PieceKind]: React.FunctionComponent<PieceProps>;
  };
} = {
  white: {
    king: WhiteKing,
    queen: WhiteQueen,
    bishop: WhiteBishop,
    knight: WhiteKnight,
    rook: WhiteRook,
    pawn: WhitePawn,
  },
  black: {
    king: BlackKing,
    queen: BlackQueen,
    bishop: BlackBishop,
    knight: BlackKnight,
    rook: BlackRook,
    pawn: BlackPawn,
  },
};

export type BoardProps = {
  data: BoardData;
  playerPieceColor: PieceColor;
  selection?: { row: number; column: number };
  onTileClick?: (row: number, column: number, piece?: Piece) => void;
};

const rectSize = 64;
const pieceSize = 60;
const pieceOffset = (rectSize - pieceSize) / 2;

const Board: React.FunctionComponent<BoardProps> = ({
  data,
  playerPieceColor,
  selection,
  onTileClick,
}: BoardProps) => {
  const boardRects: React.ReactNodeArray = [];
  const pieces: React.ReactNodeArray = [];

  for (let row = 0; row < 8; row++) {
    const rects = [];

    const rowY = (playerPieceColor === "white" ? 7 - row : row) * rectSize;

    for (let column = 0; column < 8; column++) {
      const columnX = column * rectSize;

      rects.push(
        <rect
          x={columnX}
          y={rowY}
          width={rectSize}
          height={rectSize}
          fill={(row + column) % 2 === 0 ? "#c1946e" : "#ffe1c4"}
          onClick={() =>
            onTileClick && onTileClick(row, column, data.get(row, column))
          }
        ></rect>
      );

      const piece = data.get(row, column);
      if (piece) {
        const PieceComponent = pieceMap[piece.color][piece.kind];
        pieces.push(
          <PieceComponent
            x={columnX + pieceOffset}
            y={rowY + pieceOffset}
            width={pieceSize}
            height={pieceSize}
            onClick={() =>
              onTileClick && onTileClick(row, column, data.get(row, column))
            }
          />
        );
      }
    }

    boardRects.push(rects);
  }

  let selectionRect = null;
  if (selection) {
    selectionRect = (
      <rect
        fill="#fd0"
        fillOpacity={0.7}
        x={selection.column * rectSize}
        y={selection.row * rectSize}
        width={rectSize}
        height={rectSize}
      />
    );
  }

  return (
    <svg
      className="w-full h-full"
      viewBox={`0 0 ${8 * rectSize} ${8 * rectSize}`}
    >
      {boardRects}
      {selectionRect}
      {pieces}
    </svg>
  );
};

export default Board;
