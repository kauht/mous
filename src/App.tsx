import { useState, useEffect } from "react";
import { invoke } from '@tauri-apps/api/core';
import "./App.css";

function App() {
    // Use useEffect to call setup only once when component mounts

    function record() {
        console.log("Attempting to record...");
        invoke('set_record')
            .then(() => console.log("Recording started"))
            .catch(err => console.error("Error starting recording:", err));
    }
    function replay() {
        console.log("Attempting to replay...");
        invoke('set_replay')
            .then(() => console.log("Replay started"))
            .catch(err => console.error("Error starting replay:", err));
    }

  return (
    <main className="container">
      <h1>CLICK SHIT</h1>
        <div>
            <button type="submit" onClick={record}>RECORD/STOP</button>
        </div>
        <div>
            <button type="submit" onClick={replay}>replay</button>
        </div>
    </main>
  );
}

export default App;
