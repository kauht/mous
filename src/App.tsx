import { useState, useEffect } from "react";
import { invoke } from '@tauri-apps/api/core';
import "./App.css";

function App() {
    function record() {
      invoke('set_record').then(() => console.log("Recording started"))
    }
    function replay() {
        invoke('set_replay').then(() => console.log("Replay started"))
    }

  return (
    <main className="container">
      <h1>CLICK SHIT</h1>
        <div>
            <button type="submit" onClick={record}>RECORD/STOP</button>
        </div>
        <div>
            <button className="" type="submit" onClick={replay}>replay</button>
        </div>
    </main>
  );
}

export default App;
