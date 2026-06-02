import { usePlayerStore } from "../stores/playerStore";

function fileName(path: string) {
  return path.replace(/^.*[\\/]/, "");
}

function formatTime(ms: number) {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

export function Playlist() {
  const {
    playlist,
    current_playlist_index,
    loop_mode,
    playFromPlaylist,
    removeFromPlaylist,
    setLoopMode,
    scanFolder,
    clearPlaylist,
  } = usePlayerStore();

  const cycleLoopMode = () => {
    const modes = ["none", "single", "list"];
    const idx = modes.indexOf(loop_mode);
    const next = modes[(idx + 1) % modes.length];
    console.log("[Playlist] cycleLoopMode:", loop_mode, "→", next);
    setLoopMode(next);
  };

  const loopLabel = loop_mode === "none" ? "→" : loop_mode === "single" ? "↻1" : "↻";
  const loopTitle = loop_mode === "none" ? "No Loop" : loop_mode === "single" ? "Single Loop" : "List Loop";

  return (
    <div className="w-full max-w-md flex flex-col gap-2">
      {/* Toolbar */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-400 shrink-0">
          Playlist ({playlist.length})
        </span>
        <div className="flex gap-1 ml-auto">
          <button
            onClick={cycleLoopMode}
            className="w-8 px-2 py-1 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 cursor-pointer transition-colors text-center"
            title={loopTitle}
          >
            {loopLabel}
          </button>
          <button
            onClick={scanFolder}
            className="px-2 py-1 text-xs rounded bg-gray-700 text-gray-300 hover:bg-gray-600 cursor-pointer"
            title="Add folder"
          >
            +Folder
          </button>
          <button
            onClick={clearPlaylist}
            className="px-2 py-1 text-xs rounded bg-gray-700 text-gray-300 hover:bg-red-700 cursor-pointer"
            title="Clear playlist"
          >
            Clear
          </button>
        </div>
      </div>

      {/* List */}
      <div className="max-h-64 overflow-y-auto bg-gray-900 rounded border border-gray-700">
        {playlist.length === 0 ? (
          <div className="p-3 text-xs text-gray-500 text-center">
            No tracks added. Open files or drag a folder.
          </div>
        ) : (
          playlist.map((entry, i) => (
            <div
              key={`${entry.path}-${i}`}
              onClick={() => playFromPlaylist(i)}
              className={`flex items-center gap-2 px-3 py-1.5 text-xs cursor-pointer border-b border-gray-800 last:border-0 transition-colors ${
                current_playlist_index === i
                  ? "bg-blue-900/50 text-blue-200"
                  : "text-gray-300 hover:bg-gray-800"
              }`}
            >
              <span className="text-gray-500 w-5 text-right shrink-0">
                {current_playlist_index === i ? "▶" : i + 1}
              </span>
              <span className="truncate flex-1">{fileName(entry.path)}</span>
              <span className="text-gray-500 shrink-0">{formatTime(entry.duration_ms)}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  removeFromPlaylist(i);
                }}
                className="text-gray-500 hover:text-red-400 cursor-pointer shrink-0"
                title="Remove"
              >
                ×
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
