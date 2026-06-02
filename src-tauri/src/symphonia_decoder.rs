use std::path::Path;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use crate::decoder::{Decoder, DecoderError};

pub struct SymphoniaDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    duration_ms: u64,
    total_samples: u64,
    timebase_den: u64,
}

impl SymphoniaDecoder {
    pub fn open(path: &Path) -> Result<Self, DecoderError> {
        let file = std::fs::File::open(path)?;
        // 增大缓冲区以提高兼容性（默认 4KB 可能不够）
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // 根据扩展名给 probe 提示，提高检测准确率
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probe = symphonia::default::get_probe();
        let format =
            probe.probe(&hint, mss, format_opts, metadata_opts)
                .map_err(|e| DecoderError::UnsupportedFormat(format!("{}", e)))?;

        let track = format
            .tracks()
            .iter()
            .find(|t| {
                t.codec_params
                    .as_ref()
                    .map_or(false, |p| matches!(p, CodecParameters::Audio(_)))
            })
            .ok_or_else(|| DecoderError::UnsupportedFormat("no audio track found".into()))?;

        let track_id = track.id;

        let audio_params = match track.codec_params.as_ref() {
            Some(CodecParameters::Audio(p)) => p,
            _ => return Err(DecoderError::UnsupportedFormat("not an audio track".into())),
        };

        let sample_rate = audio_params.sample_rate.unwrap_or(44100);
        let channels = audio_params
            .channels
            .as_ref()
            .map(|ch| ch.count() as u16)
            .unwrap_or(2);

        let timebase_den = track
            .time_base
            .map(|tb| tb.denom.get() as u64)
            .unwrap_or(1);

        let duration_ms = track
            .time_base
            .zip(track.duration)
            .and_then(|(tb, dur)| {
                use symphonia::core::units::Timestamp;
                tb.calc_time(Timestamp::new(dur.get() as i64))
            })
            .map(|t| {
                let total_nanos = t.as_nanos() as u128;
                (total_nanos / 1_000_000) as u64
            })
            .or_else(|| {
                track.num_frames.map(|frames| {
                    (frames * 1000) / sample_rate as u64
                })
            })
            .unwrap_or(0);

        log::info!(
            "Opened audio file: {} Hz, {} ch, {} ms, tb_den={}",
            sample_rate, channels, duration_ms, timebase_den
        );

        let codec = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
            .map_err(|e| DecoderError::UnsupportedFormat(format!("{}", e)))?;

        Ok(Self {
            format,
            decoder: codec,
            track_id,
            sample_rate,
            channels,
            duration_ms,
            total_samples: 0,
            timebase_den,
        })
    }
}

impl Decoder for SymphoniaDecoder {
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize, DecoderError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => {
                    log::info!("Decoder: end of stream (no more packets)");
                    return Err(DecoderError::EndOfStream);
                }
                Err(SymphoniaError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    log::info!("Decoder: end of stream (EOF)");
                    return Err(DecoderError::EndOfStream);
                }
                Err(e) => {
                    log::error!("Decoder: packet read error: {}", e);
                    return Err(DecoderError::Decode(format!("{}", e)));
                }
            };

            if packet.track_id != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let mut vec = Vec::new();
                    audio_buf.copy_to_vec_interleaved::<f32>(&mut vec);
                    let n = vec.len().min(output.len());
                    output[..n].copy_from_slice(&vec[..n]);
                    self.total_samples += n as u64;
                    return Ok(n);
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(e) => {
                    log::error!("Decoder: decode error: {}", e);
                    return Err(DecoderError::Decode(format!("{}", e)));
                }
            }
        }
    }

    fn seek(&mut self, time_ms: u64) -> Result<(), DecoderError> {
        let seconds = (time_ms / 1000) as i64;
        let nanos = ((time_ms % 1000) * 1_000_000) as u32;
        let seek_ts = Time::try_new(seconds, nanos).ok_or_else(|| {
            DecoderError::SeekError("invalid seek time".into())
        })?;

        log::info!("Decoder: seeking to {} ms", time_ms);

        let seeked_to = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: seek_ts,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| DecoderError::SeekError(format!("{}", e)))?;

        self.decoder.reset();

        let actual_seconds = seeked_to.actual_ts.get() as f64 / self.timebase_den as f64;
        let actual_ms = (actual_seconds * 1000.0) as u64;
        self.total_samples =
            (actual_ms * self.sample_rate as u64 * self.channels as u64) / 1000;

        log::info!("Decoder: seeked to actual {} ms, samples={}", actual_ms, self.total_samples);

        Ok(())
    }

    fn duration(&self) -> Result<u64, DecoderError> {
        Ok(self.duration_ms)
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn total_decoded_samples(&self) -> u64 {
        self.total_samples
    }
}
