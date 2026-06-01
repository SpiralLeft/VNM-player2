interface ProgressBarProps {
  positionMs: number;
  durationMs: number;
  onSeek: (positionMs: number) => void;
}

function formatTime(ms: number) {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

export function ProgressBar({ positionMs, durationMs, onSeek }: ProgressBarProps) {
  const hasFile = durationMs > 0;

  return (
    <div className="w-full max-w-md flex items-center gap-3">
      <span className="text-xs text-gray-400 w-10 text-right tabular-nums">
        {formatTime(positionMs)}
      </span>
      <input
        type="range"
        min={0}
        max={durationMs}
        value={positionMs}
        disabled={!hasFile}
        onChange={(e) => onSeek(Number(e.target.value))}
        className="w-full h-1 accent-blue-500 cursor-pointer disabled:opacity-30 disabled:cursor-default"
      />
      <span className="text-xs text-gray-400 w-10 tabular-nums">
        {formatTime(durationMs)}
      </span>
    </div>
  );
}
