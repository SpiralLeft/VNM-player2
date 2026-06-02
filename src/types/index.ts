export type PlaybackState = "idle" | "playing" | "paused";

export interface FileInfo {
  path: string;
  duration_ms: number;
  sample_rate: number;
  channels: number;
  device_sample_rate: number;
}

export interface PlaylistEntry {
  path: string;
  duration_ms: number;
}

export interface PlaylistState {
  entries: PlaylistEntry[];
  currentIndex: number | null;
  loopMode: string;
}

export interface PlayerState {
  state: PlaybackState;
  position_ms: number;
  duration_ms: number;
  current_file: string | null;
  volume: number;
}

export interface ProgressEvent {
  position_ms: number;
  duration_ms: number;
}

export interface StateChangedEvent {
  state: PlaybackState;
}
