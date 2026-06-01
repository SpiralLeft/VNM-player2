use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::Consumer;
use ringbuf::HeapCons;

struct SendStream(cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioEngine {
    stream: Option<SendStream>,
    consumer: Arc<Mutex<Option<HeapCons<f32>>>>,
    volume: Arc<AtomicU32>,
    /// 音频设备实际使用的采样率
    pub device_rate: u32,
    /// 音频设备实际使用的声道数
    #[allow(dead_code)]
    pub device_channels: u16,
}

impl AudioEngine {
    /// 创建音频引擎，使用设备的默认采样率。
    /// 返回的 `device_rate` 和 `device_channels` 是设备实际使用的参数。
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No audio output device found".to_string())?;

        let default_cfg = device
            .default_output_config()
            .map_err(|e| format!("Default config error: {}", e))?;

        let config = cpal::StreamConfig {
            channels: default_cfg.channels(),
            sample_rate: default_cfg.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let device_rate = config.sample_rate.0;
        let device_channels = config.channels;

        log::info!(
            "Audio device: {}Hz {}ch (default config)",
            device_rate, device_channels
        );

        let consumer: Arc<Mutex<Option<HeapCons<f32>>>> = Arc::new(Mutex::new(None));
        let volume = Arc::new(AtomicU32::new(f32::to_bits(1.0)));

        let stream = Self::build_stream(&device, &config, &consumer, &volume)?;

        Ok(Self {
            stream: Some(SendStream(stream)),
            consumer,
            volume,
            device_rate,
            device_channels,
        })
    }

    fn build_stream(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        consumer: &Arc<Mutex<Option<HeapCons<f32>>>>,
        volume: &Arc<AtomicU32>,
    ) -> Result<cpal::Stream, String> {
        let c = Arc::clone(consumer);
        let v = Arc::clone(volume);
        device
            .build_output_stream(
                config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    let vol = f32::from_bits(v.load(Ordering::Relaxed));
                    let mut guard = c.lock().unwrap();
                    if let Some(ref mut cons) = *guard {
                        let mut got = 0;
                        let need = data.len();
                        while got < need {
                            let n = cons.pop_slice(&mut data[got..]);
                            if n == 0 {
                                break;
                            }
                            got += n;
                        }
                        drop(guard);
                        for i in 0..need {
                            data[i] = if i < got { data[i] * vol } else { 0.0 };
                        }
                    } else {
                        drop(guard);
                        data.fill(0.0);
                    }
                },
                |err| log::error!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| format!("{}", e))
    }

    pub fn set_consumer(&self, new_consumer: HeapCons<f32>) {
        let mut guard = self.consumer.lock().unwrap();
        *guard = Some(new_consumer);
    }

    pub fn set_volume(&self, vol: f32) {
        self.volume
            .store(f32::to_bits(vol.clamp(0.0, 1.0)), Ordering::Relaxed);
    }

    pub fn start(&self) -> Result<(), String> {
        if let Some(ref send_stream) = self.stream {
            send_stream.0.play().map_err(|e| format!("{}", e))?;
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        if let Some(ref send_stream) = self.stream {
            send_stream.0.pause().map_err(|e| format!("{}", e))?;
        }
        Ok(())
    }
}
