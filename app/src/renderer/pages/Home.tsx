import { ipcRenderer } from "electron";
import React, { useState } from "react";
import { AppMessage, AppState } from "../state";

export type HomePageProps = {
  state: AppState;
  dispatch: React.Dispatch<AppMessage>;
};

const HomePage = ({ state }: HomePageProps) => {
  const [challengedPeerId, setChallengedPeerId] = useState("");

  return (
    <div className="flex flex-col items-center space-y-4 bg-primary text-white p-4 h-screen">
      <div className="flex-initial text-4xl text-center">
        InterPlanetary Chess
      </div>

      <div className="flex-initial h-2"></div>

      <div className="flex-initial flex-col text-center text-gray-300">
        <div>Your node ID</div>
        <div>
          <span className="font-bold">{state.node.id}</span>
        </div>
      </div>

      <div className="flex-initial h-6"></div>

      <div className="flex-none bg-primary-light text-white flex flex-col w-1/2 p-4 space-y-4 rounded shadow-2xl overflow-y-auto break-all">
        <div className="flex flex-col space-y-2">
          <div className="flex-initial font-bold text-xl">Challenge Peer</div>

          <div className="flex flex-col space-y-4">
            <div>
              <div>Peer ID</div>
              <div>
                <input
                  className="rounded text-black focus:outline-none md:w-full lg:w-1/2"
                  type="text"
                  value={challengedPeerId}
                  onChange={(event) => setChallengedPeerId(event.target.value)}
                ></input>
              </div>
            </div>

            <div>
              <button
                className="bg-light text-primary focus:outline-none hover:bg-light-light font-bold rounded p-1"
                onClick={() => {
                  ipcRenderer.send("challenge.send", challengedPeerId);
                  setChallengedPeerId("");
                }}
              >
                Challenge
              </button>
            </div>

            <div className="flex flex-col space-y-2">
              <div className="flex-initial font-bold text-xl">
                Requests Sent
              </div>

              <div>
                {Object.keys(state.sentChallenges).length > 0 ? (
                  Object.entries(state.sentChallenges).map(
                    ([peerId, _challenge]) => (
                      <div className="flex flex-col space-y-2">
                        <div>
                          <span className="font-bold">{peerId}</span>
                        </div>

                        <button
                          className="bg-primary text-white focus:outline-none hover:bg-light-light font-bold rounded p-1 self-start"
                          onClick={() =>
                            ipcRenderer.send("challenge.cancel", peerId)
                          }
                        >
                          Cancel
                        </button>
                      </div>
                    )
                  )
                ) : (
                  <div>No requests sent yet.</div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="flex-none bg-primary-light text-white flex flex-col w-1/2 p-4 space-y-4 rounded shadow-2xl overflow-y-auto break-all">
        <div className="font-bold text-xl">Peer Challenges</div>
        {Object.keys(state.receivedChallenges).length > 0 ? (
          Object.entries(state.receivedChallenges).map(
            ([peerId, _challenge]) => (
              <div className="flex flex-col space-y-2">
                <div>
                  Challenge from peer:{" "}
                  <span className="font-bold">{peerId}</span>
                </div>

                <div className="flex flex-row space-x-2">
                  <button
                    className="bg-light text-primary focus:outline-none hover:bg-light-light font-bold rounded p-1"
                    onClick={() => ipcRenderer.send("challenge.accept", peerId)}
                  >
                    Accept
                  </button>

                  <button
                    className="bg-primary text-white focus:outline-none hover:bg-light-light font-bold rounded p-1"
                    onClick={() =>
                      ipcRenderer.send("challenge.decline", peerId)
                    }
                  >
                    Decline
                  </button>
                </div>
              </div>
            )
          )
        ) : (
          <div>No challenges yet</div>
        )}
      </div>
    </div>
  );
};

export default HomePage;
