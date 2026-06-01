import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  PlaybackState,
  FileInfo,
  ProgressEvent,
  StateChangedEvent,
} from "../types";

interface PlayerStore {
  state: PlaybackState;
  position_ms: number;
  duration_ms: number;
  current_file: string | null;
  volume: number;

  openFile: () => Promise<void>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (positionMs: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;
  _initListeners: () => Promise<UnlistenFn[]>;
}

export const usePlayerStore = create<PlayerStore>((set) => ({
  state: "idle",
  position_ms: 0,
  duration_ms: 0,
  current_file: null,
  volume: 1.0,

  openFile: async () => {
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Audio Files",
          extensions: ["mp3", "wav", "ogg", "flac"],
        },
      ],
    });
    if (!path) return;

    const info: FileInfo = await invoke("open_file", { path });
    set({
      current_file: info.path,
      duration_ms: info.duration_ms,
      state: "paused",
      position_ms: 0,
    });
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
      await listen("playback-ended", () => {
        set({ state: "paused", position_ms: 0 });
      }),
    );

    return unlisteners;
  },
}));
