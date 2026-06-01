interface FileOpenerProps {
  currentFile: string | null;
  durationMs: number;
  onOpen: () => void;
}

function formatDuration(ms: number) {
  const totalSec = Math.floor(ms / 1000);
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function fileName(path: string) {
  return path.replace(/^.*[\\/]/, "");
}

export function FileOpener({ currentFile, durationMs, onOpen }: FileOpenerProps) {
  return (
    <div className="flex flex-col items-center gap-2 w-full max-w-md">
      <button
        onClick={onOpen}
        className="px-6 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg font-medium transition-colors cursor-pointer"
      >
        Open Audio File
      </button>
      {currentFile && (
        <div className="text-sm text-gray-400 text-center">
          <div className="text-gray-200 truncate max-w-xs">
            {fileName(currentFile)}
          </div>
          <div>Duration: {formatDuration(durationMs)}</div>
        </div>
      )}
    </div>
  );
}
