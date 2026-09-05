import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function fileName(path: string) {
  return path.replace(/^.*[\\/]/, "");
}

function formatTime(ms: number) {
  const totalSec = Math.max(0, Math.floor(ms / 1000));
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

interface TrackInfoProps {
  currentFile: string | null;
  durationMs: number;
}

export function TrackInfo({ currentFile, durationMs }: TrackInfoProps) {
  const [coverUrl, setCoverUrl] = useState<string | null>(null);

  useEffect(() => {
    if (currentFile) {
      invoke("get_cover_art", { path: currentFile })
        .then((url) => setCoverUrl(url as string | null))
        .catch(() => setCoverUrl(null));
    } else {
      setCoverUrl(null);
    }
  }, [currentFile]);

  return (
    <div className="flex flex-col items-center gap-3 w-full">
      {/* Cover art — large, ~60% of left panel width */}
      <div className="w-[60%] aspect-square rounded-lg bg-gray-800 flex items-center justify-center overflow-hidden">
        {coverUrl ? (
          <img src={coverUrl} alt="Cover" className="w-full h-full object-cover" />
        ) : (
          <svg className="w-1/3 h-1/3 text-gray-600" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
          </svg>
        )}
      </div>

      {/* Song info below cover, centered */}
      <div className="flex flex-col items-center min-w-0 w-full">
        {currentFile ? (
          <>
            <span className="text-sm text-gray-200 truncate max-w-full text-center">
              {fileName(currentFile)}
            </span>
            <span className="text-xs text-gray-500">{formatTime(durationMs)}</span>
          </>
        ) : (
          <span className="text-sm text-gray-500">No file loaded</span>
        )}
      </div>
    </div>
  );
}
