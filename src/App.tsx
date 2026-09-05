import { useEffect } from "react";
import { usePlayerStore } from "./stores/playerStore";
import { TrackInfo } from "./components/TrackInfo";
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
    play,
    pause,
    stop,
    seek,
    setVolume,
    nextTrack,
    previousTrack,
    _initListeners,
  } = usePlayerStore();

  useEffect(() => {
    const p = _initListeners();
    return () => {
      p.then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  return (
    <div className="h-screen bg-gray-950 text-white flex flex-col p-4 gap-3 select-none max-w-4xl mx-auto">
      {/* ── Main content: fills remaining height ── */}
      <div className="flex-1 min-h-0 flex gap-4">
        {/* Left: Track info, centered vertically */}
        <div className="flex-1 min-w-0 flex items-center justify-center">
          <TrackInfo currentFile={current_file} durationMs={duration_ms} />
        </div>

        {/* Right: Playlist, fills column */}
        <div className="w-80 shrink-0 flex flex-col min-h-0">
          <Playlist />
        </div>
      </div>

      {/* ── Bottom bar: Controls | Progress | Volume ── */}
      <div className="w-full flex items-center gap-3 shrink-0">
        <PlayerControls
          state={state}
          onPlay={play}
          onPause={pause}
          onStop={stop}
          onPrevious={previousTrack}
          onNext={nextTrack}
        />
        <ProgressBar positionMs={position_ms} durationMs={duration_ms} onSeek={seek} />
        <VolumeControl volume={volume} onVolumeChange={setVolume} />
      </div>
    </div>
  );
}

export default App;
