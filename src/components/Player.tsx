import type { PlaybackState } from "../types";

interface PlayerControlsProps {
  state: PlaybackState;
  onPlay: () => void;
  onPause: () => void;
  onStop: () => void;
}

export function PlayerControls({
  state,
  onPlay,
  onPause,
  onStop,
}: PlayerControlsProps) {
  const isIdle = state === "idle";
  const isPlaying = state === "playing";

  return (
    <div className="flex items-center gap-4">
      {isPlaying ? (
        <button
          onClick={onPause}
          className="w-14 h-14 bg-gray-700 hover:bg-gray-600 rounded-full flex items-center justify-center transition-colors cursor-pointer"
          title="Pause"
        >
          <svg className="w-6 h-6" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="4" width="4" height="16" />
            <rect x="14" y="4" width="4" height="16" />
          </svg>
        </button>
      ) : (
        <button
          onClick={onPlay}
          disabled={isIdle}
          className="w-14 h-14 bg-gray-700 hover:bg-gray-600 rounded-full flex items-center justify-center transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-default"
          title="Play"
        >
          <svg className="w-6 h-6 ml-1" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="6,3 20,12 6,21" />
          </svg>
        </button>
      )}

      <button
        onClick={onStop}
        disabled={isIdle}
        className="w-10 h-10 bg-gray-700 hover:bg-gray-600 rounded-full flex items-center justify-center transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-default"
        title="Stop"
      >
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
          <rect x="4" y="4" width="16" height="16" />
        </svg>
      </button>
    </div>
  );
}
