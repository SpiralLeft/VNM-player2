import { useEffect } from "react";
import { usePlayerStore } from "./stores/playerStore";
import { FileOpener } from "./components/FileOpener";
import { PlayerControls } from "./components/Player";
import { Playlist } from "./components/Playlist";
import { ProgressBar } from "./components/ProgressBar";
import { VolumeControl } from "./components/VolumeControl";

function App() {
  const {
    state,
    position_ms,
    duration_ms,
    current_file,
    volume,
    openFile,
    play,
    pause,
    stop,
    seek,
    setVolume,
    _initListeners,
  } = usePlayerStore();

  useEffect(() => {
    const p = _initListeners();
    return () => {
      p.then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  return (
    <div className="min-h-screen bg-gray-950 text-white flex flex-col items-center justify-center gap-6 p-8 select-none">
      <h1 className="text-2xl font-bold tracking-tight text-gray-100">
        VNM Player
      </h1>

      <FileOpener
        currentFile={current_file}
        durationMs={duration_ms}
        onOpen={openFile}
      />

      <ProgressBar
        positionMs={position_ms}
        durationMs={duration_ms}
        onSeek={seek}
      />

      <PlayerControls
        state={state}
        onPlay={play}
        onPause={pause}
        onStop={stop}
      />

      <VolumeControl volume={volume} onVolumeChange={setVolume} />

      <Playlist />
    </div>
  );
}

export default App;
