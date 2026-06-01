use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ringbuf::traits::{Producer, Split};
use ringbuf::HeapRb;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::audio_engine::AudioEngine;
use crate::decoder::{Decoder, DecoderError};
use crate::resampler::LinearResampler;
use crate::symphonia_decoder::SymphoniaDecoder;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayerState {
    Idle,
    #[serde(rename = "paused")]
    Loaded,
    Playing,
    #[serde(rename = "paused")]
    Paused,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileMetadata {
    pub path: String,
    pub duration_ms: u64,
    pub file_sample_rate: u32,
    pub file_channels: u16,
    pub device_sample_rate: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub position_ms: u64,
    pub duration_ms: u64,
}

pub struct Player {
    state: PlayerState,
    decoder: Option<Arc<Mutex<SymphoniaDecoder>>>,
    file_meta: Option<FileMetadata>,
    audio_engine: Option<AudioEngine>,
    stop_flag: Option<Arc<AtomicBool>>,
    app_handle: Option<AppHandle>,
    position_ms: u64,
}

impl Player {
    pub fn new() -> Self {
        Self {
            state: PlayerState::Idle,
            decoder: None,
            file_meta: None,
            audio_engine: None,
            stop_flag: None,
            app_handle: None,
            position_ms: 0,
        }
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    pub fn state(&self) -> PlayerState {
        self.state.clone()
    }

    pub fn position_ms(&self) -> u64 {
        self.position_ms
    }

    pub fn file_meta(&self) -> Option<FileMetadata> {
        self.file_meta.clone()
    }

    // ══════════════════════════════════════════════════════════════
    // 打开文件
    // ══════════════════════════════════════════════════════════════

    pub fn open_file(&mut self, path: &str) -> Result<FileMetadata, String> {
        self.kill_decoder_thread();

        let decoder = SymphoniaDecoder::open(Path::new(path))
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let file_rate = decoder.sample_rate();
        let file_ch = decoder.channels();

        // 用设备默认采样率创建引擎
        let engine = AudioEngine::new()
            .map_err(|e| format!("Failed to init audio engine: {}", e))?;
        let device_rate = engine.device_rate;

        log::info!(
            "File: {}Hz {}ch → Device: {}Hz (resampling {:.3}x)",
            file_rate, file_ch, device_rate,
            device_rate as f64 / file_rate as f64
        );

        let meta = FileMetadata {
            path: path.to_string(),
            duration_ms: decoder.duration().unwrap_or(0),
            file_sample_rate: file_rate,
            file_channels: file_ch,
            device_sample_rate: device_rate,
        };

        self.decoder = Some(Arc::new(Mutex::new(decoder)));
        self.file_meta = Some(meta.clone());
        self.audio_engine = Some(engine);
        self.position_ms = 0;
        self.state = PlayerState::Loaded;

        Ok(meta)
    }

    // ══════════════════════════════════════════════════════════════
    // 播放
    // ══════════════════════════════════════════════════════════════

    pub fn play(&mut self) -> Result<(), String> {
        match self.state {
            PlayerState::Loaded | PlayerState::Idle => {
                let pos = self.position_ms;
                if let Some(ref decoder) = self.decoder {
                    decoder.lock().unwrap().seek(pos).ok();
                }
                self.spawn_decoder_thread()?;
                self.start_engine();
                self.state = PlayerState::Playing;
                self.emit_state();
                log::info!("Playback started at {}ms", pos);
                Ok(())
            }
            PlayerState::Paused => {
                self.start_engine();
                self.state = PlayerState::Playing;
                self.emit_state();
                log::info!("Playback resumed");
                Ok(())
            }
            PlayerState::Playing => {
                self.kill_decoder_thread();
                if let Some(ref decoder) = self.decoder {
                    decoder.lock().unwrap().seek(0).ok();
                }
                self.position_ms = 0;
                self.spawn_decoder_thread()?;
                self.start_engine();
                self.emit_state();
                self.emit_progress();
                log::info!("Playback restarted after natural end");
                Ok(())
            }
        }
    }

    // ══════════════════════════════════════════════════════════════
    // 暂停
    // ══════════════════════════════════════════════════════════════

    pub fn pause(&mut self) -> Result<(), String> {
        if self.state == PlayerState::Playing {
            self.stop_engine();
            self.state = PlayerState::Paused;
            self.emit_state();
            log::info!("Playback paused");
            Ok(())
        } else {
            Err("Not playing".into())
        }
    }

    // ══════════════════════════════════════════════════════════════
    // 停止
    // ══════════════════════════════════════════════════════════════

    pub fn stop(&mut self) -> Result<(), String> {
        log::info!("Stopping playback");
        self.kill_decoder_thread();

        if let Some(ref decoder) = self.decoder {
            decoder.lock().unwrap().seek(0).ok();
        }

        // 重建引擎清除硬件缓冲区
        if let Some(ref meta) = self.file_meta {
            self.audio_engine = Some(
                AudioEngine::new()
                    .map_err(|e| format!("Failed to recreate engine: {}", e))?,
            );
            log::info!("Engine recreated ({}Hz)", meta.device_sample_rate);
        }

        self.position_ms = 0;
        self.state = PlayerState::Loaded;
        self.emit_state();
        self.emit_progress();
        Ok(())
    }

    // ══════════════════════════════════════════════════════════════
    // Seek
    // ══════════════════════════════════════════════════════════════

    pub fn seek(&mut self, position_ms: u64) -> Result<(), String> {
        let was_playing = self.state == PlayerState::Playing;
        log::info!("Seeking to {}ms (was_playing={})", position_ms, was_playing);

        self.kill_decoder_thread();

        if let Some(ref decoder) = self.decoder {
            decoder
                .lock()
                .unwrap()
                .seek(position_ms)
                .map_err(|e| format!("Seek failed: {}", e))?;
        }

        self.position_ms = position_ms;
        self.emit_progress();

        self.spawn_decoder_thread()?;

        if was_playing {
            self.start_engine();
            self.state = PlayerState::Playing;
            self.emit_state();
        }

        Ok(())
    }

    // ══════════════════════════════════════════════════════════════
    // 音量
    // ══════════════════════════════════════════════════════════════

    pub fn set_volume(&self, volume: f64) {
        if let Some(ref engine) = self.audio_engine {
            engine.set_volume(volume as f32);
        }
    }

    // ══════════════════════════════════════════════════════════════
    // 内部：启动解码线程（含重采样）
    // ══════════════════════════════════════════════════════════════

    fn spawn_decoder_thread(&mut self) -> Result<(), String> {
        let decoder = self.decoder.clone().ok_or_else(|| "No decoder".to_string())?;
        let meta = self.file_meta.clone().ok_or_else(|| "No file metadata".to_string())?;
        let file_rate = meta.file_sample_rate;
        let file_ch = meta.file_channels;
        let device_rate = meta.device_sample_rate;

        // 环形缓冲区按设备采样率计算容量
        let ring_cap = (device_rate as usize * file_ch as usize) / 2;
        let rb = HeapRb::<f32>::new(ring_cap);
        let (mut prod, cons) = rb.split();

        if let Some(ref engine) = self.audio_engine {
            engine.set_consumer(cons);
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_for_thread = Arc::clone(&stop_flag);

        let app_handle = self.app_handle.clone();
        let duration_ms = meta.duration_ms;

        std::thread::spawn(move || {
            let ch = file_ch as u64;

            // 创建重采样器：文件采样率 → 设备采样率
            let mut resampler = LinearResampler::new(file_rate, device_rate, file_ch);
            // 输出缓冲区（重采样后，按设备速率）
            let mut resample_buf = vec![0.0f32; 8192];

            let mut last_progress = Instant::now();
            let mut first_decode = true;

            loop {
                if stop_flag_for_thread.load(Ordering::SeqCst) {
                    log::info!("Decoder: stop flag");
                    break;
                }

                let mut temp = vec![0.0f32; 16384]; // 增大缓冲区，防止截断

                let n = {
                    let mut dec = decoder.lock().unwrap();
                    match dec.read_samples(&mut temp) {
                        Ok(n) => n,
                        Err(DecoderError::EndOfStream) => {
                            log::info!("Decoder: EOS");
                            0
                        }
                        Err(e) => {
                            log::error!("Decoder: error: {}", e);
                            0
                        }
                    }
                };

                if n == 0 {
                    // 冲刷重采样器中残留的样本
                    let flushed = resampler.flush(&mut resample_buf);
                    if flushed > 0 {
                        push_to_ring(&mut prod, &resample_buf[..flushed], &stop_flag_for_thread);
                    }
                    break;
                }

                // ★ 重采样
                let resampled = resampler.process(&temp[..n], &mut resample_buf);
                push_to_ring(&mut prod, &resample_buf[..resampled], &stop_flag_for_thread);

                // 进度：用文件采样率计算（因为 total_decoded_samples 是文件采样率下的样本数）
                let total = decoder.lock().unwrap().total_decoded_samples();
                let pos = if ch > 0 && file_rate > 0 {
                    (total * 1000) / (file_rate as u64 * ch)
                } else {
                    0
                };

                if first_decode || last_progress.elapsed() >= Duration::from_millis(250) {
                    first_decode = false;
                    last_progress = Instant::now();
                    if let Some(ref h) = app_handle {
                        let _ = h.emit(
                            "playback-progress",
                            ProgressEvent {
                                position_ms: pos,
                                duration_ms,
                            },
                        );
                    }
                }
            }

            if !stop_flag_for_thread.load(Ordering::SeqCst) {
                log::info!("Decoder: finished naturally");
                if let Some(ref h) = app_handle {
                    let _ = h.emit("playback-ended", ());
                }
            }
        });

        self.stop_flag = Some(stop_flag);
        log::info!("Decoder thread spawned (ring_cap={}, resampling {}→{}Hz)",
            ring_cap, file_rate, device_rate);

        Ok(())
    }

    // ══════════════════════════════════════════════════════════════
    // 引擎控制
    // ══════════════════════════════════════════════════════════════

    fn start_engine(&self) {
        if let Some(ref engine) = self.audio_engine {
            engine.start().ok();
        }
    }

    fn stop_engine(&self) {
        if let Some(ref engine) = self.audio_engine {
            engine.stop().ok();
        }
    }

    fn kill_decoder_thread(&mut self) {
        if let Some(ref flag) = self.stop_flag.take() {
            flag.store(true, Ordering::SeqCst);
        }
        self.stop_engine();
    }

    // ══════════════════════════════════════════════════════════════
    // 事件
    // ══════════════════════════════════════════════════════════════

    fn emit_state(&self) {
        if let Some(ref handle) = self.app_handle {
            #[derive(Clone, Serialize)]
            struct StatePayload { state: PlayerState }
            let _ = handle.emit("playback-state-changed", StatePayload {
                state: self.state.clone(),
            });
        }
    }

    fn emit_progress(&self) {
        if let Some(ref handle) = self.app_handle {
            let duration_ms = self.file_meta.as_ref().map(|m| m.duration_ms).unwrap_or(0);
            let _ = handle.emit("playback-progress", ProgressEvent {
                position_ms: self.position_ms,
                duration_ms,
            });
        }
    }
}

/// 将数据推入环形缓冲区，同时响应停止信号
fn push_to_ring(
    prod: &mut <HeapRb<f32> as Split>::Prod,
    data: &[f32],
    stop_flag: &AtomicBool,
) {
    let mut written = 0;
    while written < data.len() {
        if stop_flag.load(Ordering::SeqCst) {
            return;
        }
        let pushed = prod.push_slice(&data[written..]);
        written += pushed;
        if pushed == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
