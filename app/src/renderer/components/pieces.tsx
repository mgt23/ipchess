import React, { ReactSVGElement } from "react";

import whiteQueenSvgUrl from "url:../../assets/pieces/white-queen.svg";
import whiteKingSvgUrl from "url:../../assets/pieces/white-king.svg";
import whiteBishopSvgUrl from "url:../../assets/pieces/white-bishop.svg";
import whiteKnightSvgUrl from "url:../../assets/pieces/white-knight.svg";
import whiteRookSvgUrl from "url:../../assets/pieces/white-rook.svg";
import whitePawnSvgUrl from "url:../../assets/pieces/white-pawn.svg";
import blackQueenSvgUrl from "url:../../assets/pieces/black-queen.svg";
import blackKingSvgUrl from "url:../../assets/pieces/black-king.svg";
import blackBishopSvgUrl from "url:../../assets/pieces/black-bishop.svg";
import blackKnightSvgUrl from "url:../../assets/pieces/black-knight.svg";
import blackRookSvgUrl from "url:../../assets/pieces/black-rook.svg";
import blackPawnSvgUrl from "url:../../assets/pieces/black-pawn.svg";

export type PieceProps = {
  x: number;
  y: number;
  width: number;
  height: number;

  onClick?: React.MouseEventHandler<SVGSVGElement>;
};

const createPieceComponent =
  (imageUrl: string): React.FunctionComponent<PieceProps> =>
  (props: PieceProps) => {
    return (
      <svg
        viewBox="0 0 45 45"
        x={props.x}
        y={props.y}
        width={props.width}
        height={props.height}
        onClick={props.onClick}
      >
        <image href={imageUrl} />
      </svg>
    );
  };

export const WhiteQueen = createPieceComponent(whiteQueenSvgUrl);
export const WhiteKing = createPieceComponent(whiteKingSvgUrl);
export const WhiteBishop = createPieceComponent(whiteBishopSvgUrl);
export const WhiteKnight = createPieceComponent(whiteKnightSvgUrl);
export const WhiteRook = createPieceComponent(whiteRookSvgUrl);
export const WhitePawn = createPieceComponent(whitePawnSvgUrl);

export const BlackQueen = createPieceComponent(blackQueenSvgUrl);
export const BlackKing = createPieceComponent(blackKingSvgUrl);
export const BlackBishop = createPieceComponent(blackBishopSvgUrl);
export const BlackKnight = createPieceComponent(blackKnightSvgUrl);
export const BlackRook = createPieceComponent(blackRookSvgUrl);
export const BlackPawn = createPieceComponent(blackPawnSvgUrl);
