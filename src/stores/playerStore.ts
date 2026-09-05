import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  PlaybackState,
  FileInfo,
  PlaylistState,
  PlaylistEntry,
  ProgressEvent,
  StateChangedEvent,
} from "../types";

interface PlayerStore {
  state: PlaybackState;
  position_ms: number;
  duration_ms: number;
  current_file: string | null;
  volume: number;
  playlist: PlaylistEntry[];
  current_playlist_index: number | null;
  loop_mode: string;

  openFile: () => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (positionMs: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;

  // Playlist
  playFromPlaylist: (index: number) => Promise<void>;
  removeFromPlaylist: (index: number) => Promise<void>;
  nextTrack: () => Promise<void>;
  previousTrack: () => Promise<void>;
  setLoopMode: (mode: string) => Promise<void>;
  scanFolder: () => Promise<void>;
  clearPlaylist: () => Promise<void>;
  refreshPlaylist: () => Promise<void>;

  _initListeners: () => Promise<UnlistenFn[]>;
}

export const usePlayerStore = create<PlayerStore>((set, get) => ({
  state: "idle",
  position_ms: 0,
  duration_ms: 0,
  current_file: null,
  volume: 1.0,
  playlist: [],
  current_playlist_index: null,
  loop_mode: "none",

  openFile: async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "Audio Files", extensions: ["mp3", "wav", "ogg", "flac", "nwa"] }],
    });
    if (!path) return;

    try { await invoke("stop"); } catch { /* ignore */ }

    try {
      // 先加入播放列表，再打开文件（open_file 会更新 duration）
      await invoke("add_to_playlist", { path });
      const info: FileInfo = await invoke("open_file", { path });
      const pl: PlaylistState = await invoke("get_playlist");
      set({
        current_file: info.path,
        duration_ms: info.duration_ms,
        state: "paused",
        position_ms: 0,
        playlist: pl.entries,
        current_playlist_index: pl.currentIndex,
        loop_mode: pl.loopMode,
      });
    } catch (e) {
      alert(`Failed to open file:\n${e}`);
      console.error("open_file error:", e);
    }
  },

  play: async () => {
    await invoke("play");
    set({ state: "playing" });
  },

  pause: async () => {
    await invoke("pause");
    set({ state: "paused" });
  },

  stop: async () => {
    await invoke("stop");
    set({ state: "paused", position_ms: 0 });
  },

  seek: async (positionMs: number) => {
    await invoke("seek", { positionMs });
    set({ position_ms: positionMs });
  },

  setVolume: async (volume: number) => {
    await invoke("set_volume", { volume });
    set({ volume });
  },

  // ── Playlist actions ──────────────────────────────

  playFromPlaylist: async (index: number) => {
    const wasPlaying = get().state === "playing";
    const info: FileInfo = await invoke("play_from_playlist", { index });
    const pl: PlaylistState = await invoke("get_playlist");
    set({
      current_file: info.path,
      duration_ms: info.duration_ms,
      state: wasPlaying ? "playing" : "paused",
      position_ms: 0,
      playlist: pl.entries,
      current_playlist_index: pl.currentIndex,
      loop_mode: pl.loopMode,
    });
    if (wasPlaying) await invoke("play");
  },

  removeFromPlaylist: async (index: number) => {
    const pl: PlaylistState = await invoke("remove_from_playlist", { index });
    set({ playlist: pl.entries, current_playlist_index: pl.currentIndex });
  },

  nextTrack: async () => {
    const wasPlaying = get().state === "playing";
    const result = await invoke("next_track") as FileInfo | null;
    const pl: PlaylistState = await invoke("get_playlist");
    if (result) {
      set({
        current_file: result.path,
        duration_ms: result.duration_ms,
        state: wasPlaying ? "playing" : "paused",
        position_ms: 0,
        playlist: pl.entries,
        current_playlist_index: pl.currentIndex,
      });
      if (wasPlaying) await invoke("play");
    } else {
      set({ playlist: pl.entries, current_playlist_index: pl.currentIndex });
    }
  },

  previousTrack: async () => {
    const wasPlaying = get().state === "playing";
    const result = await invoke("previous_track") as FileInfo | null;
    const pl: PlaylistState = await invoke("get_playlist");
    if (result) {
      set({
        current_file: result.path,
        duration_ms: result.duration_ms,
        state: wasPlaying ? "playing" : "paused",
        position_ms: 0,
        playlist: pl.entries,
        current_playlist_index: pl.currentIndex,
      });
      if (wasPlaying) await invoke("play");
    } else {
      set({ playlist: pl.entries, current_playlist_index: pl.currentIndex });
    }
  },

  setLoopMode: async (mode: string) => {
    console.log("[Store] setLoopMode called with:", mode);
    const pl: PlaylistState = await invoke("set_loop_mode", { mode });
    console.log("[Store] setLoopMode response:", JSON.stringify(pl));
    set({ loop_mode: pl.loopMode });
    console.log("[Store] state updated to:", pl.loopMode);
  },

  scanFolder: async () => {
    const path = await open({ directory: true });
    if (!path) return;
    const pl: PlaylistState = await invoke("scan_folder", { path });
    set({ playlist: pl.entries });
  },

  clearPlaylist: async () => {
    const pl: PlaylistState = await invoke("clear_playlist");
    set({ playlist: pl.entries, current_playlist_index: null });
  },

  refreshPlaylist: async () => {
    const pl: PlaylistState = await invoke("get_playlist");
    set({
      playlist: pl.entries,
      current_playlist_index: pl.currentIndex,
      loop_mode: pl.loopMode,
    });
  },

  // ── Event listeners ───────────────────────────────

  _initListeners: async () => {
    const unlisteners: UnlistenFn[] = [];

    unlisteners.push(
      await listen<ProgressEvent>("playback-progress", (event) => {
        set({ position_ms: event.payload.position_ms });
      }),
    );

    unlisteners.push(
      await listen<StateChangedEvent>("playback-state-changed", (event) => {
        set({ state: event.payload.state });
      }),
    );

    unlisteners.push(
      await listen("playback-ended", async () => {
        const { loop_mode, nextTrack: next } = get();
        if (loop_mode === "list") {
          // 列表循环：自动播放下一首
          await next();
          // 如果切换到新文件，开始播放
          if (get().current_file) {
            try { await invoke("play"); } catch { /* */ }
            set({ state: "playing" });
          }
        } else if (loop_mode === "single") {
          // 单曲循环由后端在解码线程中处理
          // 后端已经自动 seek 回 0 并继续解码，这里更新状态
          set({ state: "playing", position_ms: 0 });
        } else {
          set({ state: "paused", position_ms: 0 });
        }
      }),
    );

    return unlisteners;
  },
}));
