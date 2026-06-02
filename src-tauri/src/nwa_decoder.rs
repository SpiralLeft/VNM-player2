use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::decoder::{Decoder, DecoderError};

/// NWA (NWA Audio) 格式解码器。
/// NWA 是一种 DPCM 压缩的音频格式，常用于 Visual Novel 游戏。
pub struct NwaDecoder {
    channels: u16,
    bps: u16,         // 8 or 16
    sample_rate: u32,
    comp_level: i32,  // -1 (uncompressed) or 0..=5
    use_run_length: i32,
    block_count: i32,
    #[allow(dead_code)]
    sample_count: i32,
    block_size: i32,  // samples per regular block
    rest_size: i32,   // samples in last block
    #[allow(dead_code)]
    offsets: Vec<i32>,

    /// 文件读取器
    reader: BufReader<File>,
    /// 当前已解码的 block 索引（-1 = 未解码任何块）
    cur_block: i32,
    /// 当前 block 解码后的 PCM 数据（16bit 样本）
    decoded: Vec<i16>,
    /// decoded 中的读取位置（以样本计）
    decoded_pos: usize,
    /// 已解码的总样本数（用于进度计算）
    total_samples: u64,
    /// 文件总时长（毫秒）
    duration_ms: u64,
}

impl NwaDecoder {
    pub fn open(path: &Path) -> Result<Self, DecoderError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // ── 读取 44 字节 NWA 头 ──────────────────────
        let mut header = [0u8; 0x2c];
        reader
            .read_exact(&mut header)
            .map_err(|e| DecoderError::Decode(format!("Can't read NWA header: {}", e)))?;

        let channels = i16::from_le_bytes([header[0x00], header[0x01]]) as u16;
        let bps = i16::from_le_bytes([header[0x02], header[0x03]]) as u16;
        let freq = i32::from_le_bytes([header[0x04], header[0x05], header[0x06], header[0x07]]) as u32;
        let comp_level = i32::from_le_bytes([header[0x08], header[0x09], header[0x0a], header[0x0b]]);
        let _userunlength = i32::from_le_bytes([header[0x0c], header[0x0d], header[0x0e], header[0x0f]]);
        let mut block_count = i32::from_le_bytes([header[0x10], header[0x11], header[0x12], header[0x13]]);
        let data_size = i32::from_le_bytes([header[0x14], header[0x15], header[0x16], header[0x17]]);
        let comp_data_size = i32::from_le_bytes([header[0x18], header[0x19], header[0x1a], header[0x1b]]);
        let sample_count = i32::from_le_bytes([header[0x1c], header[0x1d], header[0x1e], header[0x1f]]);
        let mut block_size = i32::from_le_bytes([header[0x20], header[0x21], header[0x22], header[0x23]]);
        let mut rest_size = i32::from_le_bytes([header[0x24], header[0x25], header[0x26], header[0x27]]);

        // ── 验证 ─────────────────────────────────────
        if channels != 1 && channels != 2 {
            return Err(DecoderError::UnsupportedFormat(format!(
                "NWA: only mono/stereo supported, got {} channels", channels
            )));
        }
        if bps != 8 && bps != 16 {
            return Err(DecoderError::UnsupportedFormat(format!(
                "NWA: only 8/16 bit supported, got {} bits", bps
            )));
        }
        if comp_level < -1 || comp_level > 5 {
            return Err(DecoderError::UnsupportedFormat(format!(
                "NWA: unsupported compression level {}", comp_level
            )));
        }

        // ── 无压缩模式调整 ──────────────────────────
        if comp_level == -1 {
            block_size = 65536;
            let byps = (bps / 8) as i32;
            rest_size = (data_size % (block_size * byps)) / byps;
            let rest = if rest_size > 0 { 1 } else { 0 };
            block_count = data_size / (block_size * byps) + rest;
        }

        if block_count <= 0 || block_count > 1_000_000 {
            return Err(DecoderError::UnsupportedFormat(format!(
                "NWA: invalid block count {}", block_count
            )));
        }

        // ── 读取偏移表 ──────────────────────────────
        let mut offsets = Vec::with_capacity(block_count as usize);
        if comp_level != -1 {
            for _ in 0..block_count {
                let mut tmp = [0u8; 4];
                reader
                    .read_exact(&mut tmp)
                    .map_err(|e| DecoderError::Decode(format!("Can't read NWA offset table: {}", e)))?;
                offsets.push(i32::from_le_bytes(tmp));
            }
            // 最后一个偏移不能越界
            if offsets[block_count as usize - 1] >= comp_data_size {
                return Err(DecoderError::Decode("NWA: last offset overruns file".into()));
            }
        }

        // ── 计算时长 ─────────────────────────────────
        // sample_count 是单个采样值总数，立体声时需除以声道数
        let duration_ms = if freq > 0 && channels > 0 {
            (sample_count as u64 * 1000) / (freq as u64 * channels as u64)
        } else {
            0
        };

        eprintln!(
            "[NWA Decoder] {}Hz {}ch {}bps comp={} blocks={} samples={} duration={}ms",
            freq, channels, bps, comp_level, block_count, sample_count, duration_ms
        );

        Ok(Self {
            channels,
            bps,
            sample_rate: freq,
            comp_level,
            use_run_length: _userunlength,
            block_count,
            sample_count,
            block_size,
            rest_size,
            offsets,
            reader,
            cur_block: -1,
            decoded: Vec::new(),
            decoded_pos: 0,
            total_samples: 0,
            duration_ms,
        })
    }

    /// 解码一个 block 到 self.decoded 中。
    /// 返回解码的样本数。
    fn decode_block(&mut self) -> Result<usize, DecoderError> {
        if self.comp_level == -1 {
            return self.decode_uncompressed_block();
        }
        self.decode_compressed_block()
    }

    fn decode_uncompressed_block(&mut self) -> Result<usize, DecoderError> {
        let byps = (self.bps / 8) as usize;
        // 与压缩模式一致的 block 边界判断
        let cur_samples = if self.cur_block + 1 < self.block_count - 1 {
            self.block_size as usize
        } else if self.cur_block + 1 == self.block_count - 1 {
            self.rest_size as usize
        } else {
            return Err(DecoderError::EndOfStream);
        };
        let nbytes = cur_samples * byps;

        let mut raw = vec![0u8; nbytes];
        let n = self
            .reader
            .read(&mut raw)
            .map_err(|e| DecoderError::Decode(format!("NWA: read uncompressed block: {}", e)))?;
        if n == 0 {
            return Err(DecoderError::EndOfStream);
        }
        raw.truncate(n);

        let actual_samples = n / byps;
        self.decoded.clear();
        if self.bps == 16 {
            for chunk in raw[..actual_samples * byps].chunks_exact(2) {
                self.decoded.push(i16::from_le_bytes([chunk[0], chunk[1]]));
            }
        } else {
            for &b in &raw[..actual_samples] {
                self.decoded.push((b as i16) << 8);
            }
        }
        self.decoded_pos = 0;
        self.cur_block += 1;
        eprintln!(
            "  [NWA] block {}/{}: {} samples ({} bytes)",
            self.cur_block, self.block_count, actual_samples, n
        );
        Ok(actual_samples)
    }

    fn decode_compressed_block(&mut self) -> Result<usize, DecoderError> {
        // 确定 block 大小
        let (cur_block_samples, cur_comp_size) = if self.cur_block + 1 < self.block_count - 1
            || (self.cur_block == -1 && self.block_count > 1)
        {
            // 不是最后一个 block
            let idx = (self.cur_block + 1) as usize;
            let samples = self.block_size as usize;
            let comp = (self.offsets[idx + 1] - self.offsets[idx]) as usize;
            (samples, comp)
        } else if self.cur_block + 1 == self.block_count - 1 {
            // 最后一个 block
            let samples = self.rest_size as usize;
            // 最后一个 block 的压缩大小通过文件剩余部分计算
            let comp = (self.block_size as usize * (self.bps as usize / 8) * 2).min(0x100000);
            (samples, comp)
        } else {
            return Err(DecoderError::EndOfStream);
        };

        // 读取压缩数据
        let mut raw = vec![0u8; cur_comp_size];
        let n = self
            .reader
            .read(&mut raw)
            .map_err(|e| DecoderError::Decode(format!("NWA: read compressed block: {}", e)))?;
        raw.truncate(n);
        if n == 0 {
            return Err(DecoderError::EndOfStream);
        }

        // DPCM 解码：cur_block_samples 是 block 内的样本总数（已含立体声交错）
        self.decoded = vec![0i16; cur_block_samples];
        self.decode_dpcm(&raw, cur_block_samples);
        self.decoded_pos = 0;
        self.cur_block += 1;
        Ok(cur_block_samples)
    }

    /// DPCM 解码核心算法。
    /// 与 Go 参考实现完全一致：bitstream 从 raw[0] 开始，
    /// 初始样本值也从 bitstream 中读取但不消耗 bit 位。
    fn decode_dpcm(&mut self, raw: &[u8], out_samples: usize) {
        let channels = self.channels as usize;
        let bps = self.bps as usize;
        let complevel = self.comp_level;
        let userunlength = self.use_run_length;

        let mut d = [0i32; 2];
        let mut flipflag = 0usize;
        let mut runlength: i32 = 0;

        // 初始样本值从 raw 中读取（不影响 bitstream 位置）
        let mut init_pos = 0usize;
        for ch in 0..channels {
            if bps == 8 {
                d[ch] = raw[init_pos] as i32;
                init_pos += 1;
            } else {
                d[ch] = i16::from_le_bytes([raw[init_pos], raw[init_pos + 1]]) as i32;
                init_pos += 2;
            }
        }

        // ★ bitstream 从 raw 的第 0 字节开始（与 Go 代码一致）
        let mut byte_idx = 0usize;
        let mut bit_shift: u32 = 0;

        let output = &mut self.decoded;
        let mut out_idx = 0;

        while out_idx < out_samples {
            if runlength == 0 {
                let exponent = get_bits(raw, &mut byte_idx, &mut bit_shift, 3) as i32;

                match exponent {
                    7 => {
                        if get_bits(raw, &mut byte_idx, &mut bit_shift, 1) == 1 {
                            d[flipflag] = 0;
                        } else {
                            let (bits, shift) = if complevel >= 3 {
                                (8u32, 9u32)
                            } else {
                                (8u32 - complevel as u32, 2 + 7 + complevel as u32)
                            };
                            let mask1 = 1u32 << (bits - 1);
                            let mask2 = (1u32 << (bits - 1)) - 1;
                            let b = get_bits(raw, &mut byte_idx, &mut bit_shift, bits);
                            if b & mask1 != 0 {
                                d[flipflag] -= ((b & mask2) << shift) as i32;
                            } else {
                                d[flipflag] += ((b & mask2) << shift) as i32;
                            }
                        }
                    }
                    1..=6 => {
                        let (bits, shift) = if complevel >= 3 {
                            (complevel as u32 + 3, (1 + exponent) as u32)
                        } else {
                            (5u32 - complevel as u32, (2 + exponent as u32 + complevel as u32))
                        };
                        let mask1 = 1u32 << (bits - 1);
                        let mask2 = (1u32 << (bits - 1)) - 1;
                        let b = get_bits(raw, &mut byte_idx, &mut bit_shift, bits);
                        if b & mask1 != 0 {
                            d[flipflag] -= ((b & mask2) << shift) as i32;
                        } else {
                            d[flipflag] += ((b & mask2) << shift) as i32;
                        }
                    }
                    0 => {
                        if userunlength == 1 {
                            runlength = get_bits(raw, &mut byte_idx, &mut bit_shift, 1) as i32;
                            if runlength == 1 {
                                runlength = get_bits(raw, &mut byte_idx, &mut bit_shift, 2) as i32;
                                if runlength == 3 {
                                    runlength = get_bits(raw, &mut byte_idx, &mut bit_shift, 8) as i32;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                runlength -= 1;
            }

            // 写入样本
            if out_idx < out_samples {
                output[out_idx] = d[flipflag] as i16;
                out_idx += 1;
            }

            // 立体声：交替声道
            if channels == 2 {
                flipflag ^= 1;
            }
        }
    }

    /// 定位并解码目标样本所在的 block
    fn seek_to_sample(&mut self, target_sample: u64) -> Result<(), DecoderError> {
        if self.comp_level == -1 {
            // 无压缩：直接 seek 文件位置
            let byps = (self.bps / 8) as u64;
            let file_offset = 0x2c + target_sample * byps;
            self.reader
                .seek(SeekFrom::Start(file_offset))
                .map_err(|e| DecoderError::SeekError(format!("{}", e)))?;
            self.cur_block = -1;
            self.decoded.clear();
            self.decoded_pos = 0;
            // 重算 cur_block
            let samples_per_block = self.block_size as u64;
            self.cur_block = (target_sample / samples_per_block) as i32 - 1;
            // 解码目标 block
            self.decode_block()?;
            // 跳过 block 内的样本
            let skip = (target_sample % samples_per_block) as usize;
            self.decoded_pos = skip.min(self.decoded.len());
            self.total_samples = target_sample;
            return Ok(());
        }

        // 压缩模式：计算目标 block
        let block_idx = if self.block_size > 0 {
            (target_sample / self.block_size as u64) as i32
        } else {
            0
        };

        if block_idx != self.cur_block {
            // 需要在文件中定位到该 block 的偏移
            if block_idx >= 0 && (block_idx as usize) < self.offsets.len() {
                let offset = self.offsets[block_idx as usize] as u64;
                self.reader
                    .seek(SeekFrom::Start(offset))
                    .map_err(|e| DecoderError::SeekError(format!("{}", e)))?;
                self.cur_block = block_idx - 1;
                self.decoded.clear();
                self.decoded_pos = 0;
            } else {
                return Err(DecoderError::SeekError("block index out of range".into()));
            }
        }

        // 解码该 block
        if self.decoded.is_empty() || self.decoded_pos >= self.decoded.len() {
            self.decode_block()?;
        }

        // 跳过 block 内偏移的样本（target_sample 已是单个采样值索引）
        let skip_within = (target_sample % self.block_size as u64) as usize;
        self.decoded_pos = skip_within.min(self.decoded.len());
        self.total_samples = target_sample;

        Ok(())
    }
}

impl Decoder for NwaDecoder {
    fn read_samples(&mut self, output: &mut [f32]) -> Result<usize, DecoderError> {
        let mut written = 0;

        while written < output.len() {
            // 如果当前 block 已读完，解码下一个
            if self.decoded.is_empty() || self.decoded_pos >= self.decoded.len() {
                match self.decode_block() {
                    Ok(_) => {}
                    Err(DecoderError::EndOfStream) => break,
                    Err(e) => return Err(e),
                }
            }

            // 从当前 block 复制样本到输出
            let remaining = self.decoded.len() - self.decoded_pos;
            let to_copy = remaining.min(output.len() - written);

            for i in 0..to_copy {
                // 将 i16 转换为 f32（归一化到 [-1.0, 1.0]）
                let sample = self.decoded[self.decoded_pos + i] as f32 / 32768.0;
                output[written + i] = sample;
            }

            self.decoded_pos += to_copy;
            written += to_copy;
            self.total_samples += to_copy as u64;
        }

        if written == 0 {
            return Err(DecoderError::EndOfStream);
        }
        Ok(written)
    }

    fn seek(&mut self, time_ms: u64) -> Result<(), DecoderError> {
        // target_sample 必须是单个采样值索引（含立体声交错）
        let target_sample =
            (time_ms * self.sample_rate as u64 * self.channels as u64) / 1000;
        self.seek_to_sample(target_sample)
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

/// 从字节流中读取指定数量的 bit（最大 16bit）。
/// byte_idx: 当前字节索引（会更新）
/// bit_shift: 当前 bit 偏移（0-15，会更新）
fn get_bits(data: &[u8], byte_idx: &mut usize, bit_shift: &mut u32, bits: u32) -> u32 {
    if *bit_shift > 8 {
        *byte_idx += 1;
        *bit_shift -= 8;
    }
    // 安全边界检查
    if *byte_idx + 1 >= data.len() {
        return 0;
    }
    let val = u16::from_le_bytes([data[*byte_idx], data[*byte_idx + 1]]) as u32;
    let ret = (val >> *bit_shift) & ((1u32 << bits) - 1);
    *bit_shift += bits;
    ret
}
