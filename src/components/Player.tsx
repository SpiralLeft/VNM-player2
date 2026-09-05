import type { PlaybackState } from "../types";

interface PlayerControlsProps {
  state: PlaybackState;
  onPlay: () => void;
  onPause: () => void;
  onStop: () => void;
  onPrevious: () => void;
  onNext: () => void;
}

export function PlayerControls({
  state,
  onPlay,
  onPause,
  onStop,
  onPrevious,
  onNext,
}: PlayerControlsProps) {
  const isIdle = state === "idle";
  const isPlaying = state === "playing";
  const iconClass =
    "w-6 h-6 text-gray-300 hover:text-white transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-default";

  return (
    <div className="flex items-center gap-4">
      {/* Stop */}
      <button onClick={onStop} disabled={isIdle} title="Stop" className={iconClass}>
        <svg viewBox="0 0 24 24" fill="currentColor">
          <rect x="4" y="4" width="16" height="16" rx="1" />
        </svg>
      </button>

      {/* Previous */}
      <button onClick={onPrevious} title="Previous" className={iconClass}>
        <svg viewBox="0 0 24 24" fill="currentColor">
          <polygon points="19,4 8,12 19,20" />
          <rect x="5" y="4" width="2" height="16" />
        </svg>
      </button>

      {/* Play / Pause */}
      {isPlaying ? (
        <button onClick={onPause} title="Pause" className={iconClass}>
          <svg viewBox="0 0 24 24" fill="currentColor">
            <rect x="5" y="3" width="5" height="18" rx="1" />
            <rect x="14" y="3" width="5" height="18" rx="1" />
          </svg>
        </button>
      ) : (
        <button onClick={onPlay} disabled={isIdle} title="Play" className={iconClass}>
          <svg viewBox="0 0 24 24" fill="currentColor">
            <polygon points="6,3 20,12 6,21" />
          </svg>
        </button>
      )}

      {/* Next */}
      <button onClick={onNext} title="Next" className={iconClass}>
        <svg viewBox="0 0 24 24" fill="currentColor">
          <polygon points="5,4 16,12 5,20" />
          <rect x="17" y="4" width="2" height="16" />
        </svg>
      </button>
    </div>
  );
}
