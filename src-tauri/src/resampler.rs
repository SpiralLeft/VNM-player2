/// 线性插值重采样器。
/// 将音频从输入采样率转换到输出采样率。
///
/// 算法：跟踪当前在输入流中的位置（以输出帧为单位），
/// 每当跨过整数输出帧边界时，用线性插值产生一个输出帧。
pub struct LinearResampler {
    /// 每个输入帧对应多少输出帧：output_rate / input_rate
    /// 例：44100→48000 时 ratio = 48000/44100 ≈ 1.0889
    step: f64,
    /// 当前累积位置（以输出帧为单位）
    pos: f64,
    /// 上一个输入帧的值（每声道），用于线性插值
    prev: Vec<f32>,
    has_prev: bool,
    channels: usize,
}

impl LinearResampler {
    pub fn new(input_rate: u32, output_rate: u32, channels: u16) -> Self {
        Self {
            step: output_rate as f64 / input_rate as f64,
            pos: 0.0,
            prev: vec![0.0f32; channels as usize],
            has_prev: false,
            channels: channels as usize,
        }
    }

    /// 处理输入缓冲区，写入输出缓冲区。返回写入的样本数。
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) -> usize {
        let ch = self.channels;
        let input_frames = input.len() / ch;
        let max_out_frames = output.len() / ch;
        let mut out_frame = 0;

        for in_frame in 0..input_frames {
            let in_off = in_frame * ch;
            let cur: Vec<f32> = (0..ch).map(|c| input[in_off + c]).collect();

            if !self.has_prev {
                // 第一个输入帧：直接作为第一个输出帧
                if out_frame < max_out_frames {
                    output[out_frame * ch..][..ch].copy_from_slice(&cur);
                    out_frame += 1;
                }
                self.prev.copy_from_slice(&cur);
                self.has_prev = true;
                self.pos += self.step; // 跳过第一个输入帧覆盖的时间
                continue;
            }

            // 在 prev 帧和 cur 帧之间，pos 会跨过若干整数输出帧位置
            // pos_before + step = pos_after → 检查跨过了哪些整数
            let pos_before = self.pos - self.step;
            let pos_at_cur = self.pos;

            // floor(pos_before)+1, floor(pos_before)+2, ..., floor(pos_at_cur)
            let first_out = (pos_before.floor() as i64 + 1).max(0) as usize;
            let last_out = pos_at_cur.floor() as usize;

            for out_idx in first_out..=last_out {
                if out_frame >= max_out_frames {
                    break;
                }
                // 该输出帧在输入帧之间的位置（0=prev, 1=cur）
                let t = ((out_idx as f64 - pos_before) / self.step).clamp(0.0, 1.0) as f32;
                let out_off = out_frame * ch;
                for c in 0..ch {
                    output[out_off + c] =
                        self.prev[c] + (cur[c] - self.prev[c]) * t;
                }
                out_frame += 1;
            }

            self.prev.copy_from_slice(&cur);
            self.pos += self.step;
        }

        out_frame * ch
    }

    /// 冲刷最后残留的样本
    pub fn flush(&mut self, output: &mut [f32]) -> usize {
        if !self.has_prev || output.len() < self.channels {
            return 0;
        }
        output[..self.channels].copy_from_slice(&self.prev[..self.channels]);
        self.has_prev = false;
        self.pos = 0.0;
        self.channels
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.has_prev = false;
    }
}
