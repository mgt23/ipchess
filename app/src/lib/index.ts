export type PieceColor = "white" | "black";

export type PieceKind =
  | "king"
  | "queen"
  | "bishop"
  | "knight"
  | "rook"
  | "pawn";

export type Piece = {
  color: PieceColor;
  kind: PieceKind;
};

const piecesInitialOrder: PieceKind[] = [
  "rook",
  "knight",
  "bishop",
  "queen",
  "king",
  "bishop",
  "knight",
  "rook",
];

export class BoardData {
  private data: Array<Array<Piece | null>>;

  constructor() {
    this.data = [];
    for (let i = 0; i < 8; i++) {
      this.data.push(new Array(8).fill(null));
    }

    for (let column = 0; column < 8; column++) {
      this.data[0][column] = {
        color: "white",
        kind: piecesInitialOrder[column],
      };
      this.data[1][column] = { color: "white", kind: "pawn" };

      this.data[7][column] = {
        color: "black",
        kind: piecesInitialOrder[column],
      };
      this.data[6][column] = { color: "black", kind: "pawn" };
    }
  }

  get(row: number, col: number): Piece | null {
    return this.data[row][col];
  }
}
