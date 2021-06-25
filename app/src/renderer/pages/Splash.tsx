import React from "react";

const SplashPage: React.FunctionComponent = () => {
  return (
    <div className="bg-primary text-white flex flex-col space-y-4 h-screen justify-center items-center">
      <svg
        className="animate-spinner h-12 w-12"
        viewBox="0 0 100 100"
        xmlns="http://www.w3.org/2000/svg"
      >
        <circle className="stroke-current text-accent" cx="50" cy="50" r="45" />
      </svg>

      <div className={"text-lg"}>Connecting</div>
    </div>
  );
};

export default SplashPage;
