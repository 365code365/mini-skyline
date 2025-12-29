//! video 组件 - 视频播放器
//! 
//! 使用 symphonia 解码音频，手动解析 MP4 容器，openh264 解码 H.264 视频
//! 
//! 属性：
//! - src: 视频资源地址
//! - autoplay: 是否自动播放
//! - loop: 是否循环播放
//! - muted: 是否静音
//! - controls: 是否显示控制条

use super::base::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::{Canvas, Color, Paint, PaintStyle, Path, Rect as GeoRect};
use taffy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use std::path::PathBuf;
use std::fs::File;

// 音频播放
use rodio::{OutputStream, Sink, Source};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

// H.264 解码
use openh264::decoder::Decoder as H264Decoder;

// 音频播放器 (thread_local 因为 OutputStream 不是 Send)
thread_local! {
    static AUDIO_STREAM: std::cell::RefCell<Option<(OutputStream, Sink)>> = std::cell::RefCell::new(None);
}

/// 视频帧数据
pub struct VideoFrame {
    pub data: Vec<u8>,  // RGBA 数据
    pub width: u32,
    pub height: u32,
    pub timestamp: f64, // 秒
}

/// 音频样本缓冲
struct AudioBuffer {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

impl AudioBuffer {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            sample_rate: 44100,
            channels: 2,
        }
    }
}

/// 视频播放器状态
pub struct VideoPlayer {
    pub src: String,
    pub file_path: Option<PathBuf>,
    pub frames: Vec<VideoFrame>,
    pub current_frame: usize,
    pub fps: f64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub is_playing: bool,
    pub is_loaded: bool,
    pub play_start_time: Option<Instant>,
    pub play_start_frame: usize,
    pub loop_play: bool,
    pub load_error: Option<String>,
    pub muted: bool,
    // 音频数据
    audio_buffer: Option<AudioBuffer>,
}

impl VideoPlayer {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.to_string(),
            file_path: None,
            frames: Vec::new(),
            current_frame: 0,
            fps: 24.0,
            duration: 0.0,
            width: 0,
            height: 0,
            is_playing: false,
            is_loaded: false,
            play_start_time: None,
            play_start_frame: 0,
            loop_play: false,
            load_error: None,
            muted: false,
            audio_buffer: None,
        }
    }
    
    /// 加载视频
    pub fn load(&mut self) -> Result<(), String> {
        // 尝试多个可能的路径
        let paths_to_try = vec![
            self.src.clone(),
            self.src.trim_start_matches('/').to_string(),
            format!("sample-app{}", self.src),
            format!("sample-app/{}", self.src.trim_start_matches('/')),
        ];
        
        let mut actual_path = None;
        for path in &paths_to_try {
            if std::path::Path::new(path).exists() {
                actual_path = Some(PathBuf::from(path));
                break;
            }
        }
        
        let path = actual_path.ok_or_else(|| format!("Cannot find video: {}", self.src))?;
        self.file_path = Some(path.clone());
        
        println!("🎬 Loading video: {}", path.display());
        
        // 解析 MP4 并解码视频帧
        self.decode_video_manual(&path)?;
        
        // 解码音频
        if let Err(e) = self.decode_audio(&path) {
            println!("   ⚠️ Audio decode warning: {}", e);
        }
        
        Ok(())
    }
    
    /// 手动解析 MP4 并解码视频帧
    fn decode_video_manual(&mut self, path: &PathBuf) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
        
        // 解析 MP4 box 结构
        let mut pos = 0;
        let mut width = 0u32;
        let mut height = 0u32;
        let mut timescale = 1u32;
        let mut sample_sizes: Vec<u32> = Vec::new();
        let mut chunk_offsets: Vec<u64> = Vec::new();
        let mut sample_to_chunk: Vec<(u32, u32, u32)> = Vec::new(); // (first_chunk, samples_per_chunk, sample_desc_idx)
        let mut sync_samples: Vec<u32> = Vec::new();
        let mut sample_durations: Vec<(u32, u32)> = Vec::new(); // (count, delta)
        
        while pos + 8 <= data.len() {
            let box_size = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
            let box_type = &data[pos+4..pos+8];
            
            if box_size < 8 || pos + box_size > data.len() {
                break;
            }
            
            match box_type {
                b"tkhd" => {
                    // Track header - 获取宽高
                    if box_size >= 92 {
                        let version = data[pos + 8];
                        let offset = if version == 1 { pos + 84 } else { pos + 76 };
                        if offset + 8 <= data.len() {
                            width = u32::from_be_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) >> 16;
                            height = u32::from_be_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]) >> 16;
                        }
                    }
                }
                b"mdhd" => {
                    // Media header - 获取 timescale
                    let version = data[pos + 8];
                    let ts_offset = if version == 1 { pos + 28 } else { pos + 20 };
                    if ts_offset + 4 <= data.len() {
                        timescale = u32::from_be_bytes([data[ts_offset], data[ts_offset+1], data[ts_offset+2], data[ts_offset+3]]);
                    }
                }
                b"stts" => {
                    // Time-to-sample
                    if pos + 16 <= data.len() {
                        let entry_count = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]) as usize;
                        let mut off = pos + 16;
                        for _ in 0..entry_count {
                            if off + 8 > data.len() { break; }
                            let count = u32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                            let delta = u32::from_be_bytes([data[off+4], data[off+5], data[off+6], data[off+7]]);
                            sample_durations.push((count, delta));
                            off += 8;
                        }
                    }
                }
                b"stss" => {
                    // Sync sample (keyframes)
                    if pos + 16 <= data.len() {
                        let entry_count = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]) as usize;
                        let mut off = pos + 16;
                        for _ in 0..entry_count {
                            if off + 4 > data.len() { break; }
                            let sample_num = u32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                            sync_samples.push(sample_num);
                            off += 4;
                        }
                    }
                }
                b"stsc" => {
                    // Sample-to-chunk
                    if pos + 16 <= data.len() {
                        let entry_count = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]) as usize;
                        let mut off = pos + 16;
                        for _ in 0..entry_count {
                            if off + 12 > data.len() { break; }
                            let first_chunk = u32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                            let samples_per_chunk = u32::from_be_bytes([data[off+4], data[off+5], data[off+6], data[off+7]]);
                            let sample_desc_idx = u32::from_be_bytes([data[off+8], data[off+9], data[off+10], data[off+11]]);
                            sample_to_chunk.push((first_chunk, samples_per_chunk, sample_desc_idx));
                            off += 12;
                        }
                    }
                }
                b"stsz" => {
                    // Sample sizes
                    if pos + 20 <= data.len() {
                        let default_size = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]);
                        let sample_count = u32::from_be_bytes([data[pos+16], data[pos+17], data[pos+18], data[pos+19]]) as usize;
                        if default_size == 0 {
                            let mut off = pos + 20;
                            for _ in 0..sample_count {
                                if off + 4 > data.len() { break; }
                                let size = u32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                                sample_sizes.push(size);
                                off += 4;
                            }
                        } else {
                            sample_sizes = vec![default_size; sample_count];
                        }
                    }
                }
                b"stco" => {
                    // Chunk offsets (32-bit)
                    if pos + 16 <= data.len() {
                        let entry_count = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]) as usize;
                        let mut off = pos + 16;
                        for _ in 0..entry_count {
                            if off + 4 > data.len() { break; }
                            let offset = u32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]) as u64;
                            chunk_offsets.push(offset);
                            off += 4;
                        }
                    }
                }
                b"co64" => {
                    // Chunk offsets (64-bit)
                    if pos + 16 <= data.len() {
                        let entry_count = u32::from_be_bytes([data[pos+12], data[pos+13], data[pos+14], data[pos+15]]) as usize;
                        let mut off = pos + 16;
                        for _ in 0..entry_count {
                            if off + 8 > data.len() { break; }
                            let offset = u64::from_be_bytes([
                                data[off], data[off+1], data[off+2], data[off+3],
                                data[off+4], data[off+5], data[off+6], data[off+7]
                            ]);
                            chunk_offsets.push(offset);
                            off += 8;
                        }
                    }
                }
                _ => {}
            }
            
            // 递归进入容器 box
            if matches!(box_type, b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl") {
                pos += 8;
                continue;
            }
            
            pos += box_size;
        }
        
        self.width = width;
        self.height = height;
        
        let sample_count = sample_sizes.len();
        if sample_count == 0 {
            return Err("No samples found".to_string());
        }
        
        // 计算时长和帧率
        let mut total_duration = 0u64;
        for (count, delta) in &sample_durations {
            total_duration += *count as u64 * *delta as u64;
        }
        self.duration = total_duration as f64 / timescale as f64;
        self.fps = if self.duration > 0.0 { sample_count as f64 / self.duration } else { 24.0 };
        
        println!("   Video: {}x{}, {:.1} fps, {:.1}s, {} samples", 
            self.width, self.height, self.fps, self.duration, sample_count);
        
        // 获取 SPS/PPS
        let (sps, pps) = self.extract_sps_pps_manual(path)?;
        
        // 创建 H.264 解码器
        let mut decoder = H264Decoder::new()
            .map_err(|e| format!("H264 decoder init error: {:?}", e))?;
        
        // 构建样本偏移表
        let sample_offsets = self.build_sample_offsets(&sample_sizes, &chunk_offsets, &sample_to_chunk);
        
        // 计算每个样本的时间戳
        let timestamps = self.build_timestamps(&sample_durations, timescale);
        
        // 解码帧
        let start_time = Instant::now();
        let mut decoded_count = 0;
        let mut skipped_count = 0;
        
        for (sample_idx, &(offset, size)) in sample_offsets.iter().enumerate() {
            if offset as usize + size as usize > data.len() {
                continue;
            }
            
            let sample_data = &data[offset as usize..(offset as usize + size as usize)];
            let timestamp = timestamps.get(sample_idx).copied().unwrap_or(0.0);
            let is_sync = sync_samples.is_empty() || sync_samples.contains(&((sample_idx + 1) as u32));
            
            // 构建 NAL 单元
            let mut nal_data = Vec::new();
            
            if is_sync {
                nal_data.extend_from_slice(&[0, 0, 0, 1]);
                nal_data.extend_from_slice(&sps);
                nal_data.extend_from_slice(&[0, 0, 0, 1]);
                nal_data.extend_from_slice(&pps);
            }
            
            // 解析 AVCC 格式
            let mut off = 0;
            while off + 4 <= sample_data.len() {
                let nal_size = u32::from_be_bytes([
                    sample_data[off], sample_data[off+1], sample_data[off+2], sample_data[off+3]
                ]) as usize;
                off += 4;
                
                if off + nal_size <= sample_data.len() {
                    nal_data.extend_from_slice(&[0, 0, 0, 1]);
                    nal_data.extend_from_slice(&sample_data[off..off + nal_size]);
                    off += nal_size;
                } else {
                    break;
                }
            }
            
            // 解码
            match decoder.decode(&nal_data) {
                Ok(Some(yuv)) => {
                    let rgba = self.yuv_to_rgba(&yuv);
                    self.frames.push(VideoFrame {
                        data: rgba,
                        width: self.width,
                        height: self.height,
                        timestamp,
                    });
                    decoded_count += 1;
                }
                Ok(None) => skipped_count += 1,
                Err(_) => skipped_count += 1,
            }
            
            if decoded_count >= 3600 {
                println!("   ⚠️ Reached max frame limit");
                break;
            }
        }
        
        let decode_time = start_time.elapsed();
        
        if !self.frames.is_empty() {
            self.is_loaded = true;
            println!("✅ Video loaded: {} frames decoded, {} skipped ({:.1}s)", 
                decoded_count, skipped_count, decode_time.as_secs_f64());
        } else {
            return Err(format!("No frames decoded (skipped {})", skipped_count));
        }
        
        Ok(())
    }
    
    /// 构建样本偏移表
    fn build_sample_offsets(&self, sample_sizes: &[u32], chunk_offsets: &[u64], sample_to_chunk: &[(u32, u32, u32)]) -> Vec<(u64, u32)> {
        let mut result = Vec::new();
        let mut sample_idx = 0;
        
        for (chunk_idx, &chunk_offset) in chunk_offsets.iter().enumerate() {
            let chunk_num = (chunk_idx + 1) as u32;
            
            // 找到这个 chunk 的 samples_per_chunk
            let mut samples_per_chunk = 1u32;
            for (i, &(first_chunk, spc, _)) in sample_to_chunk.iter().enumerate() {
                if chunk_num >= first_chunk {
                    let next_first = sample_to_chunk.get(i + 1).map(|x| x.0).unwrap_or(u32::MAX);
                    if chunk_num < next_first {
                        samples_per_chunk = spc;
                        break;
                    }
                }
            }
            
            let mut offset = chunk_offset;
            for _ in 0..samples_per_chunk {
                if sample_idx >= sample_sizes.len() {
                    break;
                }
                let size = sample_sizes[sample_idx];
                result.push((offset, size));
                offset += size as u64;
                sample_idx += 1;
            }
        }
        
        result
    }
    
    /// 构建时间戳表
    fn build_timestamps(&self, sample_durations: &[(u32, u32)], timescale: u32) -> Vec<f64> {
        let mut timestamps = Vec::new();
        let mut current_time = 0u64;
        
        for &(count, delta) in sample_durations {
            for _ in 0..count {
                timestamps.push(current_time as f64 / timescale as f64);
                current_time += delta as u64;
            }
        }
        
        timestamps
    }

    /// 从 MP4 提取 SPS/PPS (手动解析)
    fn extract_sps_pps_manual(&self, path: &PathBuf) -> Result<(Vec<u8>, Vec<u8>), String> {
        let data = std::fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
        
        // 搜索 avcC box
        let avcc_marker = b"avcC";
        let mut pos = 0;
        while pos + 4 < data.len() {
            if &data[pos..pos+4] == avcc_marker {
                // 找到 avcC，解析内容
                // avcC 格式:
                // 1 byte: configurationVersion
                // 1 byte: AVCProfileIndication
                // 1 byte: profile_compatibility
                // 1 byte: AVCLevelIndication
                // 1 byte: lengthSizeMinusOne (低2位)
                // 1 byte: numOfSequenceParameterSets (低5位)
                // 然后是 SPS 列表
                // 1 byte: numOfPictureParameterSets
                // 然后是 PPS 列表
                
                let avcc_start = pos + 4;
                if avcc_start + 6 >= data.len() {
                    pos += 1;
                    continue;
                }
                
                let num_sps = data[avcc_start + 5] & 0x1F;
                let mut offset = avcc_start + 6;
                
                let mut sps = Vec::new();
                for _ in 0..num_sps {
                    if offset + 2 > data.len() { break; }
                    let sps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + sps_len > data.len() { break; }
                    sps = data[offset..offset + sps_len].to_vec();
                    offset += sps_len;
                }
                
                if offset >= data.len() {
                    pos += 1;
                    continue;
                }
                
                let num_pps = data[offset];
                offset += 1;
                
                let mut pps = Vec::new();
                for _ in 0..num_pps {
                    if offset + 2 > data.len() { break; }
                    let pps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + pps_len > data.len() { break; }
                    pps = data[offset..offset + pps_len].to_vec();
                    offset += pps_len;
                }
                
                if !sps.is_empty() && !pps.is_empty() {
                    println!("   Found SPS ({} bytes), PPS ({} bytes) via manual parse", sps.len(), pps.len());
                    return Ok((sps, pps));
                }
            }
            pos += 1;
        }
        
        Err("Cannot find avcC in file".to_string())
    }
    
    /// YUV 转 RGBA
    fn yuv_to_rgba(&self, yuv: &openh264::decoder::DecodedYUV) -> Vec<u8> {
        use openh264::formats::YUVSource;
        
        let (width, height) = yuv.dimensions();
        let mut rgba = vec![0u8; width * height * 4];
        
        let (y_stride, u_stride, v_stride) = yuv.strides();
        
        let y_data = yuv.y();
        let u_data = yuv.u();
        let v_data = yuv.v();
        
        for row in 0..height {
            for col in 0..width {
                let y_idx = row * y_stride + col;
                let uv_row = row / 2;
                let uv_col = col / 2;
                let u_idx = uv_row * u_stride + uv_col;
                let v_idx = uv_row * v_stride + uv_col;
                
                let y = y_data.get(y_idx).copied().unwrap_or(0) as f32;
                let u = u_data.get(u_idx).copied().unwrap_or(128) as f32 - 128.0;
                let v = v_data.get(v_idx).copied().unwrap_or(128) as f32 - 128.0;
                
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
                
                let idx = (row * width + col) * 4;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
        
        rgba
    }
    
    /// 解码音频
    fn decode_audio(&mut self, path: &PathBuf) -> Result<(), String> {
        let file = File::open(path).map_err(|e| format!("Cannot open: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        
        let mut hint = Hint::new();
        hint.with_extension("mp4");
        
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Probe error: {}", e))?;
        
        let mut format = probed.format;
        
        // 查找音频轨道
        let track = format.tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or("No audio track")?;
        
        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
        
        println!("   Audio: {} Hz, {} channels", sample_rate, channels);
        
        // 创建解码器
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Decoder error: {}", e))?;
        
        let mut audio_buffer = AudioBuffer::new();
        audio_buffer.sample_rate = sample_rate;
        audio_buffer.channels = channels;
        
        // 解码所有音频包
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(ref e)) 
                    if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(_) => break,
            };
            
            if packet.track_id() != track_id {
                continue;
            }
            
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;
                    let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                    sample_buf.copy_interleaved_ref(decoded);
                    audio_buffer.samples.extend_from_slice(sample_buf.samples());
                }
                Err(_) => continue,
            }
        }
        
        if !audio_buffer.samples.is_empty() {
            let duration_secs = audio_buffer.samples.len() as f64 
                / (audio_buffer.sample_rate as f64 * audio_buffer.channels as f64);
            println!("✅ Audio loaded: {:.1}s ({} samples)", duration_secs, audio_buffer.samples.len());
            self.audio_buffer = Some(audio_buffer);
        }
        
        Ok(())
    }

    /// 获取当前帧
    pub fn get_current_frame(&mut self) -> Option<&VideoFrame> {
        if !self.is_loaded || self.frames.is_empty() {
            return None;
        }
        
        if self.is_playing {
            if let Some(start_time) = self.play_start_time {
                let elapsed = start_time.elapsed().as_secs_f64();
                let start_timestamp = self.frames.get(self.play_start_frame)
                    .map(|f| f.timestamp)
                    .unwrap_or(0.0);
                let current_time = start_timestamp + elapsed;
                
                // 找到对应时间的帧
                let mut target_frame = self.play_start_frame;
                for (i, frame) in self.frames.iter().enumerate().skip(self.play_start_frame) {
                    if frame.timestamp <= current_time {
                        target_frame = i;
                    } else {
                        break;
                    }
                }
                
                self.current_frame = target_frame;
                
                // 检查是否播放完毕
                if self.current_frame >= self.frames.len() - 1 {
                    if self.loop_play {
                        self.current_frame = 0;
                        self.play_start_frame = 0;
                        self.play_start_time = Some(Instant::now());
                        self.restart_audio();
                    } else {
                        self.current_frame = self.frames.len() - 1;
                        self.is_playing = false;
                        Self::stop_audio_static();
                    }
                }
            }
        }
        
        self.frames.get(self.current_frame)
    }
    
    /// 播放
    pub fn play(&mut self) {
        if self.is_loaded && !self.is_playing {
            self.is_playing = true;
            self.play_start_time = Some(Instant::now());
            self.play_start_frame = self.current_frame;
            self.start_audio();
        }
    }
    
    /// 暂停
    pub fn pause(&mut self) {
        self.is_playing = false;
        self.play_start_time = None;
        Self::stop_audio_static();
    }
    
    /// 开始播放音频
    fn start_audio(&self) {
        if self.muted { return; }
        
        let audio_buffer = match &self.audio_buffer {
            Some(ab) => ab,
            None => return,
        };
        
        Self::stop_audio_static();
        
        // 计算跳过的样本数
        let skip_time = self.frames.get(self.current_frame)
            .map(|f| f.timestamp)
            .unwrap_or(0.0);
        let skip_samples = (skip_time * audio_buffer.sample_rate as f64 * audio_buffer.channels as f64) as usize;
        
        // 创建音频源
        let samples: Vec<f32> = if skip_samples < audio_buffer.samples.len() {
            audio_buffer.samples[skip_samples..].to_vec()
        } else {
            audio_buffer.samples.clone()
        };
        
        let sample_rate = audio_buffer.sample_rate;
        let channels = audio_buffer.channels;
        
        match OutputStream::try_default() {
            Ok((stream, stream_handle)) => {
                match Sink::try_new(&stream_handle) {
                    Ok(sink) => {
                        let source = SamplesSource::new(samples, sample_rate, channels);
                        sink.append(source);
                        sink.play();
                        
                        AUDIO_STREAM.with(|cell| {
                            *cell.borrow_mut() = Some((stream, sink));
                        });
                        
                        println!("🔊 Audio playback started");
                    }
                    Err(e) => println!("❌ Sink error: {:?}", e),
                }
            }
            Err(e) => println!("❌ Audio output error: {:?}", e),
        }
    }
    
    fn stop_audio_static() {
        AUDIO_STREAM.with(|cell| {
            if let Some((_, ref sink)) = *cell.borrow() {
                sink.stop();
            }
            *cell.borrow_mut() = None;
        });
    }
    
    fn restart_audio(&self) {
        Self::stop_audio_static();
        
        let audio_buffer = match &self.audio_buffer {
            Some(ab) => ab,
            None => return,
        };
        
        let samples = audio_buffer.samples.clone();
        let sample_rate = audio_buffer.sample_rate;
        let channels = audio_buffer.channels;
        
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let source = SamplesSource::new(samples, sample_rate, channels);
                sink.append(source);
                sink.play();
                AUDIO_STREAM.with(|cell| {
                    *cell.borrow_mut() = Some((stream, sink));
                });
            }
        }
    }
}

/// 自定义音频源，用于播放解码后的样本
struct SamplesSource {
    samples: Vec<f32>,
    position: usize,
    sample_rate: u32,
    channels: u16,
}

impl SamplesSource {
    fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        Self { samples, position: 0, sample_rate, channels }
    }
}

impl Iterator for SamplesSource {
    type Item = f32;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for SamplesSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }
    
    fn channels(&self) -> u16 {
        self.channels
    }
    
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    fn total_duration(&self) -> Option<std::time::Duration> {
        let total_samples = self.samples.len() / self.channels as usize;
        Some(std::time::Duration::from_secs_f64(
            total_samples as f64 / self.sample_rate as f64
        ))
    }
}


/// 全局视频播放器缓存
static VIDEO_PLAYERS: OnceLock<Arc<Mutex<HashMap<String, VideoPlayer>>>> = OnceLock::new();

fn get_video_players() -> &'static Arc<Mutex<HashMap<String, VideoPlayer>>> {
    VIDEO_PLAYERS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// 获取或创建视频播放器
pub fn get_or_create_player(src: &str, autoplay: bool, loop_play: bool) -> bool {
    let players = get_video_players();
    let mut players_guard = players.lock().unwrap();
    
    if !players_guard.contains_key(src) {
        let mut player = VideoPlayer::new(src);
        player.loop_play = loop_play;
        
        match player.load() {
            Ok(_) => {
                if autoplay {
                    player.play();
                }
                players_guard.insert(src.to_string(), player);
                return true;
            }
            Err(e) => {
                println!("❌ Failed to load video: {}", e);
                let mut player = VideoPlayer::new(src);
                player.load_error = Some(e);
                players_guard.insert(src.to_string(), player);
                return false;
            }
        }
    }
    true
}

/// 获取视频当前帧
pub fn get_video_frame(src: &str) -> Option<(Vec<u8>, u32, u32)> {
    let players = get_video_players();
    let mut players_guard = players.lock().ok()?;
    
    if let Some(player) = players_guard.get_mut(src) {
        if let Some(frame) = player.get_current_frame() {
            return Some((frame.data.clone(), frame.width, frame.height));
        }
    }
    None
}

/// 检查视频是否正在播放
pub fn is_video_playing(src: &str) -> bool {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        if let Some(player) = players_guard.get(src) {
            return player.is_playing;
        }
    }
    false
}

/// 播放/暂停视频
pub fn toggle_video_play(src: &str) {
    let players = get_video_players();
    if let Ok(mut players_guard) = players.lock() {
        if let Some(player) = players_guard.get_mut(src) {
            if player.is_playing {
                player.pause();
            } else {
                player.play();
            }
        }
    }
}

/// 获取视频进度信息
pub fn get_video_progress(src: &str) -> Option<(f64, f64)> {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        if let Some(player) = players_guard.get(src) {
            if player.is_loaded && !player.frames.is_empty() {
                let current_time = player.frames.get(player.current_frame)
                    .map(|f| f.timestamp)
                    .unwrap_or(0.0);
                return Some((current_time, player.duration));
            }
        }
    }
    None
}

/// 检查是否有任何视频正在播放
pub fn has_playing_video() -> bool {
    let players = get_video_players();
    if let Ok(players_guard) = players.lock() {
        for player in players_guard.values() {
            if player.is_playing {
                return true;
            }
        }
    }
    false
}

pub struct VideoComponent;

impl VideoComponent {
    pub fn build(node: &WxmlNode, ctx: &mut ComponentContext) -> Option<RenderNode> {
        let (mut ts, mut ns) = build_base_style(node, ctx);
        let events = extract_events(node);
        let attrs = node.attributes.clone();
        let sf = ctx.scale_factor;
        
        let src = node.get_attr("src").unwrap_or("");
        let autoplay = node.get_attr("autoplay").map(|v| v == "true" || v == "{{true}}").unwrap_or(false);
        let loop_play = node.get_attr("loop").map(|v| v == "true" || v == "{{true}}").unwrap_or(false);
        
        let default_width = 300.0;
        let default_height = 225.0;
        
        if ts.size.width == Dimension::Auto {
            ts.size.width = length(default_width * sf);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(default_height * sf);
        }
        
        ns.background_color = Some(Color::BLACK);
        ns.border_radius = 4.0 * sf;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        if !src.is_empty() {
            get_or_create_player(src, autoplay, loop_play);
        }
        
        Some(RenderNode {
            tag: "video".into(),
            text: src.into(),
            attrs,
            taffy_node: tn,
            style: ns,
            children: vec![],
            events,
        })
    }
    
    pub fn draw(
        node: &RenderNode, 
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        x: f32, y: f32, w: f32, h: f32, sf: f32
    ) {
        let style = &node.style;
        let radius = style.border_radius;
        let src = &node.text;
        
        let bg_paint = Paint::new()
            .with_color(Color::BLACK)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if radius > 0.0 {
            let mut path = Path::new();
            path.add_round_rect(x, y, w, h, radius);
            canvas.draw_path(&path, &bg_paint);
        } else {
            canvas.draw_rect(&GeoRect::new(x, y, w, h), &bg_paint);
        }
        
        if !src.is_empty() {
            if let Some((frame_data, frame_w, frame_h)) = get_video_frame(src) {
                canvas.draw_image(&frame_data, frame_w, frame_h, x, y, w, h, "aspectFit", radius);
                Self::draw_controls(canvas, text_renderer, src, x, y, w, h, sf);
                return;
            }
        }
        
        Self::draw_placeholder(canvas, x, y, w, h, sf);
    }
    
    fn draw_placeholder(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let btn_size = 50.0 * sf;
        
        let bg_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 128))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(cx, cy, btn_size / 2.0, &bg_paint);
        
        let tri_size = btn_size * 0.35;
        let mut path = Path::new();
        path.move_to(cx - tri_size * 0.4, cy - tri_size * 0.6);
        path.line_to(cx - tri_size * 0.4, cy + tri_size * 0.6);
        path.line_to(cx + tri_size * 0.6, cy);
        path.close();
        
        let tri_paint = Paint::new()
            .with_color(Color::WHITE)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_path(&path, &tri_paint);
    }
    
    fn draw_controls(
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        src: &str,
        x: f32, y: f32, w: f32, h: f32, sf: f32
    ) {
        let bar_height = 36.0 * sf;
        let bar_y = y + h - bar_height;
        
        let bg_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 160))
            .with_style(PaintStyle::Fill);
        canvas.draw_rect(&GeoRect::new(x, bar_y, w, bar_height), &bg_paint);
        
        let is_playing = is_video_playing(src);
        let btn_size = 24.0 * sf;
        let btn_x = x + 12.0 * sf;
        let btn_y = bar_y + (bar_height - btn_size) / 2.0;
        
        let btn_paint = Paint::new()
            .with_color(Color::WHITE)
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        
        if is_playing {
            let bar_w = 4.0 * sf;
            let bar_h = btn_size * 0.6;
            let gap = 6.0 * sf;
            canvas.draw_rect(&GeoRect::new(
                btn_x + (btn_size - gap - bar_w * 2.0) / 2.0,
                btn_y + (btn_size - bar_h) / 2.0,
                bar_w, bar_h
            ), &btn_paint);
            canvas.draw_rect(&GeoRect::new(
                btn_x + (btn_size + gap - bar_w * 2.0) / 2.0 + bar_w,
                btn_y + (btn_size - bar_h) / 2.0,
                bar_w, bar_h
            ), &btn_paint);
        } else {
            let tri_size = btn_size * 0.5;
            let mut path = Path::new();
            let cx = btn_x + btn_size / 2.0;
            let cy = btn_y + btn_size / 2.0;
            path.move_to(cx - tri_size * 0.3, cy - tri_size * 0.5);
            path.line_to(cx - tri_size * 0.3, cy + tri_size * 0.5);
            path.line_to(cx + tri_size * 0.5, cy);
            path.close();
            canvas.draw_path(&path, &btn_paint);
        }
        
        if let Some((current, duration)) = get_video_progress(src) {
            let progress_x = btn_x + btn_size + 12.0 * sf;
            let progress_w = w - progress_x - 80.0 * sf - x;
            let progress_h = 4.0 * sf;
            let progress_y = bar_y + (bar_height - progress_h) / 2.0;
            
            let track_paint = Paint::new()
                .with_color(Color::new(255, 255, 255, 80))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w, progress_h), &track_paint);
            
            let progress = if duration > 0.0 { current / duration } else { 0.0 };
            let fill_paint = Paint::new()
                .with_color(Color::from_hex(0x07C160))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w * progress as f32, progress_h), &fill_paint);
            
            if let Some(tr) = text_renderer {
                let time_text = format!("{} / {}", Self::format_time(current), Self::format_time(duration));
                let time_x = progress_x + progress_w + 8.0 * sf;
                let time_y = bar_y + bar_height / 2.0 - 6.0 * sf;
                let text_paint = Paint::new().with_color(Color::WHITE);
                tr.draw_text(canvas, &time_text, time_x, time_y, 11.0 * sf, &text_paint);
            }
        }
    }
    
    fn format_time(seconds: f64) -> String {
        let mins = (seconds / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        format!("{:02}:{:02}", mins, secs)
    }
}
