function formatTime(ms: number) {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

interface ProgressBarProps {
  positionMs: number;
  durationMs: number;
  onSeek: (positionMs: number) => void;
}

export function ProgressBar({ positionMs, durationMs, onSeek }: ProgressBarProps) {
  const hasFile = durationMs > 0;

  return (
    <div className="flex items-center flex-1 min-w-0">
      <input
        type="range"
        min={0}
        max={durationMs}
        value={positionMs}
        disabled={!hasFile}
        onChange={(e) => onSeek(Number(e.target.value))}
        className="flex-1 min-w-0 h-1 accent-blue-500 cursor-pointer disabled:opacity-30 disabled:cursor-default"
      />
      <span className="text-xs text-gray-400 tabular-nums ml-2 w-24 shrink-0 text-right">
        {formatTime(positionMs)} / {formatTime(durationMs)}
      </span>
    </div>
  );
}
