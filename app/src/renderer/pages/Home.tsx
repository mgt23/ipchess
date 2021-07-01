import { ipcRenderer } from "electron";
import React, { useState } from "react";
import { AppMessage, AppState } from "../state";

export type HomePageProps = {
  state: AppState;
  dispatch: React.Dispatch<AppMessage>;
};

const HomePage = ({ state, dispatch }: HomePageProps) => {
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

      <div className="bg-primary-light text-white flex flex-col md:w-1/2 lg:w-1/4 min-w-min p-4 space-y-4 rounded shadow-2xl">
        <div className="flex-initial font-bold text-xl">Challenge Peer</div>

        <div className="flex flex-col space-y-2">
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
              ipcRenderer.invoke("challenge.send", challengedPeerId);
              dispatch({
                type: "peer-challenged",
                payload: { peerId: challengedPeerId },
              });
            }}
          >
            Challenge
          </button>
        </div>
      </div>

      <div className="bg-primary-light text-white flex flex-col md:w-1/2 lg:w-1/4 min-w-min p-4 space-y-4 rounded shadow-2xl overflow-y-auto">
        <div className="flex-initial font-bold text-xl">Peer Challenges</div>
        {state.receivedChallenges.length > 0 ? (
          state.receivedChallenges.map((challenge) => (
            <div className="flex flex-col space-y-2">
              <div>
                Challenge from peer:{" "}
                <span className="font-bold">{challenge.peerId}</span>
              </div>

              <div className="flex flex-row space-x-2">
                <button
                  className="bg-light text-primary focus:outline-none hover:bg-light-light font-bold rounded p-1"
                  onClick={() => {
                    ipcRenderer.invoke("challenge.accept", challenge.peerId);
                  }}
                >
                  Accept
                </button>

                <button
                  className="bg-primary text-white focus:outline-none hover:bg-light-light font-bold rounded p-1"
                  onClick={() => {}}
                >
                  Decline
                </button>
              </div>
            </div>
          ))
        ) : (
          <div>No challenges yet</div>
        )}
      </div>
    </div>
  );
};

export default HomePage;
