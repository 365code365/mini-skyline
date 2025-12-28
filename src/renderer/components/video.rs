//! video 组件 - 视频播放器
//! 
//! 使用 mp4 crate 解析容器，openh264 解码 H.264 视频
//! 使用 rodio 播放音频
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
use std::io::{Read, Seek, Cursor};
use openh264::formats::YUVSource;
use rodio::{Decoder, OutputStream, Sink, Source};

/// 视频帧数据
pub struct VideoFrame {
    pub data: Vec<u8>,  // RGBA 数据
    pub width: u32,
    pub height: u32,
    pub timestamp: f64, // 秒
}

/// 全局音频播放器（使用 thread_local 因为 OutputStream 不是 Send）
thread_local! {
    static AUDIO_STREAM: std::cell::RefCell<Option<(OutputStream, Sink)>> = std::cell::RefCell::new(None);
}

/// 视频播放器状态
pub struct VideoPlayer {
    pub src: String,
    pub frames: Vec<VideoFrame>,
    pub current_frame: usize,
    pub fps: f64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub is_playing: bool,
    pub is_loaded: bool,
    pub last_frame_time: Instant,
    pub play_start_time: Option<Instant>,
    pub play_start_frame: usize,
    pub loop_play: bool,
    pub load_error: Option<String>,
    pub audio_data: Option<Vec<u8>>,
    pub muted: bool,
}

impl VideoPlayer {
    pub fn new(src: &str) -> Self {
        Self {
            src: src.to_string(),
            frames: Vec::new(),
            current_frame: 0,
            fps: 30.0,
            duration: 0.0,
            width: 0,
            height: 0,
            is_playing: false,
            is_loaded: false,
            last_frame_time: Instant::now(),
            play_start_time: None,
            play_start_frame: 0,
            loop_play: false,
            load_error: None,
            audio_data: None,
            muted: false,
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
            format!("sample-app/assets/{}", self.src.trim_start_matches("/sample-app/assets/")),
            format!("assets/{}", self.src.trim_start_matches("/assets/")),
        ];
        
        let mut file = None;
        let mut actual_path = String::new();
        
        for path in &paths_to_try {
            if std::path::Path::new(path).exists() {
                match std::fs::File::open(path) {
                    Ok(f) => {
                        file = Some(f);
                        actual_path = path.clone();
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
        
        let file = file.ok_or_else(|| format!("Cannot find video: {}", self.src))?;
        
        println!("🎬 Loading video: {}", actual_path);
        
        // 解析 MP4
        self.decode_mp4(file, &actual_path)?;
        
        Ok(())
    }
    
    /// 解码 MP4 文件
    fn decode_mp4<R: Read + Seek>(&mut self, _reader: R, path: &str) -> Result<(), String> {
        // 完全手动解析 MP4 文件
        let file_data = std::fs::read(path).map_err(|e| e.to_string())?;
        
        // 查找 avcC box 获取 SPS/PPS
        let avcc_data = self.find_avcc_box(&file_data)?;
        let (sps_list, pps_list, nal_length_size) = self.parse_avcc(&avcc_data)?;
        
        println!("   Found {} SPS, {} PPS, NAL length size: {}", 
            sps_list.len(), pps_list.len(), nal_length_size);
        
        // 解析视频尺寸和帧率
        let (width, height, timescale, duration, sample_count) = self.parse_video_info(&file_data)?;
        self.width = width;
        self.height = height;
        
        if sample_count > 0 && duration > 0 {
            self.fps = (sample_count as f64 * timescale as f64) / duration as f64;
            self.duration = duration as f64 / timescale as f64;
        }
        
        println!("   Video: {}x{}, {:.1} fps, {:.1}s, {} samples", 
            self.width, self.height, self.fps, self.duration, sample_count);
        
        // 提取音频数据
        if let Ok(audio_data) = self.extract_audio_track(&file_data) {
            println!("   🔊 Audio track extracted: {} bytes", audio_data.len());
            self.audio_data = Some(audio_data);
        } else {
            println!("   ⚠️ No audio track found or extraction failed");
        }
        
        // 初始化 H.264 解码器
        let mut decoder = openh264::decoder::Decoder::new()
            .map_err(|e| format!("Failed to create decoder: {:?}", e))?;
        
        // 发送 SPS
        for sps in &sps_list {
            let mut nal = vec![0x00, 0x00, 0x00, 0x01];
            nal.extend_from_slice(sps);
            let _ = decoder.decode(&nal);
        }
        // 发送 PPS
        for pps in &pps_list {
            let mut nal = vec![0x00, 0x00, 0x00, 0x01];
            nal.extend_from_slice(pps);
            let _ = decoder.decode(&nal);
        }
        
        // 解析样本表获取样本位置和大小
        let samples = self.parse_sample_table(&file_data)?;
        println!("   Parsed {} samples from stbl", samples.len());
        
        // 找到 mdat box 的位置
        let mdat_offset = self.find_mdat_offset(&file_data)?;
        
        // 限制解码帧数（解码全部帧，最多 3000 帧约 2 分钟 @ 24fps）
        let max_frames = 3000.min(samples.len());
        let mut decoded_count = 0;
        
        for (i, (offset, size)) in samples.iter().take(max_frames).enumerate() {
            let abs_offset = mdat_offset + *offset;
            if abs_offset + size > file_data.len() {
                continue;
            }
            
            let sample_data = &file_data[abs_offset..abs_offset + size];
            let timestamp = i as f64 / self.fps;
            
            if let Some(frame) = self.decode_h264_sample_with_nal_size(&mut decoder, sample_data, nal_length_size)? {
                self.frames.push(VideoFrame {
                    data: frame,
                    width: self.width,
                    height: self.height,
                    timestamp,
                });
                decoded_count += 1;
            }
        }
        
        if !self.frames.is_empty() {
            self.is_loaded = true;
            println!("✅ Video loaded: {} frames decoded", self.frames.len());
        } else {
            return Err(format!("No frames decoded (tried {} samples)", max_frames));
        }
        
        Ok(())
    }
    
    /// 提取音频轨道数据（返回原始 MP4 文件用于 rodio 解码）
    fn extract_audio_track(&self, _data: &[u8]) -> Result<Vec<u8>, String> {
        // rodio 的 Decoder 可以直接解码 MP4 文件中的音频
        // 我们直接返回整个文件，让 rodio 处理
        // 这是最简单的方式，因为 rodio 内部使用 symphonia 支持 MP4/AAC
        
        // 检查是否有音频轨道（查找 mp4a box）
        let has_audio = self.find_box(_data, b"mp4a").is_some() 
            || self.find_box(_data, b"esds").is_some();
        
        if has_audio {
            Ok(_data.to_vec())
        } else {
            Err("No audio track found".to_string())
        }
    }
    
    /// 解析视频信息
    fn parse_video_info(&self, data: &[u8]) -> Result<(u32, u32, u32, u64, u32), String> {
        // 查找 tkhd box 获取尺寸
        let mut width = 960u32;
        let mut height = 400u32;
        
        // 查找 "tkhd" 
        if let Some(pos) = self.find_box(data, b"tkhd") {
            // tkhd box 结构: version(1) + flags(3) + ... + width(4) + height(4) at end
            let box_start = pos - 4;
            let box_size = u32::from_be_bytes([data[box_start], data[box_start+1], data[box_start+2], data[box_start+3]]) as usize;
            let box_end = box_start + box_size;
            
            if box_end >= 8 && box_end <= data.len() {
                // width 和 height 在 tkhd 末尾，是 16.16 定点数
                let w_bytes = &data[box_end-8..box_end-4];
                let h_bytes = &data[box_end-4..box_end];
                width = (u32::from_be_bytes([w_bytes[0], w_bytes[1], w_bytes[2], w_bytes[3]]) >> 16) as u32;
                height = (u32::from_be_bytes([h_bytes[0], h_bytes[1], h_bytes[2], h_bytes[3]]) >> 16) as u32;
            }
        }
        
        // 查找 mdhd box 获取 timescale 和 duration
        let mut timescale = 24000u32;
        let mut duration = 0u64;
        
        if let Some(pos) = self.find_box(data, b"mdhd") {
            let version = data.get(pos + 4).copied().unwrap_or(0);
            if version == 0 {
                // 32-bit: skip version(1) + flags(3) + creation(4) + modification(4)
                let ts_offset = pos + 4 + 12;
                let dur_offset = ts_offset + 4;
                if dur_offset + 4 <= data.len() {
                    timescale = u32::from_be_bytes([data[ts_offset], data[ts_offset+1], data[ts_offset+2], data[ts_offset+3]]);
                    duration = u32::from_be_bytes([data[dur_offset], data[dur_offset+1], data[dur_offset+2], data[dur_offset+3]]) as u64;
                }
            } else {
                // 64-bit: skip version(1) + flags(3) + creation(8) + modification(8)
                let ts_offset = pos + 4 + 20;
                let dur_offset = ts_offset + 4;
                if dur_offset + 8 <= data.len() {
                    timescale = u32::from_be_bytes([data[ts_offset], data[ts_offset+1], data[ts_offset+2], data[ts_offset+3]]);
                    duration = u64::from_be_bytes([
                        data[dur_offset], data[dur_offset+1], data[dur_offset+2], data[dur_offset+3],
                        data[dur_offset+4], data[dur_offset+5], data[dur_offset+6], data[dur_offset+7]
                    ]);
                }
            }
        }
        
        // 查找 stsz box 获取样本数量
        let mut sample_count = 0u32;
        if let Some(pos) = self.find_box(data, b"stsz") {
            // stsz: version(1) + flags(3) + sample_size(4) + sample_count(4)
            let count_offset = pos + 4 + 8;
            if count_offset + 4 <= data.len() {
                sample_count = u32::from_be_bytes([data[count_offset], data[count_offset+1], data[count_offset+2], data[count_offset+3]]);
            }
        }
        
        Ok((width, height, timescale, duration, sample_count))
    }
    
    /// 解析样本表
    fn parse_sample_table(&self, data: &[u8]) -> Result<Vec<(usize, usize)>, String> {
        let mut samples = Vec::new();
        
        // 解析 stsz (sample sizes)
        let mut sample_sizes = Vec::new();
        if let Some(pos) = self.find_box(data, b"stsz") {
            let offset = pos + 4; // skip "stsz"
            if offset + 12 <= data.len() {
                let default_size = u32::from_be_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]) as usize;
                let count = u32::from_be_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]) as usize;
                
                if default_size > 0 {
                    sample_sizes = vec![default_size; count];
                } else {
                    let mut i = offset + 12;
                    for _ in 0..count {
                        if i + 4 > data.len() { break; }
                        let size = u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize;
                        sample_sizes.push(size);
                        i += 4;
                    }
                }
            }
        }
        
        // 解析 stco/co64 (chunk offsets)
        let mut chunk_offsets = Vec::new();
        if let Some(pos) = self.find_box(data, b"stco") {
            let offset = pos + 4;
            if offset + 8 <= data.len() {
                let count = u32::from_be_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]) as usize;
                let mut i = offset + 8;
                for _ in 0..count {
                    if i + 4 > data.len() { break; }
                    let off = u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize;
                    chunk_offsets.push(off);
                    i += 4;
                }
            }
        } else if let Some(pos) = self.find_box(data, b"co64") {
            let offset = pos + 4;
            if offset + 8 <= data.len() {
                let count = u32::from_be_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]) as usize;
                let mut i = offset + 8;
                for _ in 0..count {
                    if i + 8 > data.len() { break; }
                    let off = u64::from_be_bytes([
                        data[i], data[i+1], data[i+2], data[i+3],
                        data[i+4], data[i+5], data[i+6], data[i+7]
                    ]) as usize;
                    chunk_offsets.push(off);
                    i += 8;
                }
            }
        }
        
        // 解析 stsc (sample-to-chunk)
        let mut stsc_entries = Vec::new();
        if let Some(pos) = self.find_box(data, b"stsc") {
            let offset = pos + 4;
            if offset + 8 <= data.len() {
                let count = u32::from_be_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]) as usize;
                let mut i = offset + 8;
                for _ in 0..count {
                    if i + 12 > data.len() { break; }
                    let first_chunk = u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize;
                    let samples_per_chunk = u32::from_be_bytes([data[i+4], data[i+5], data[i+6], data[i+7]]) as usize;
                    stsc_entries.push((first_chunk, samples_per_chunk));
                    i += 12;
                }
            }
        }
        
        // 构建样本列表
        if chunk_offsets.is_empty() || sample_sizes.is_empty() {
            return Err("Missing chunk offsets or sample sizes".to_string());
        }
        
        let mut sample_idx = 0;
        let mut stsc_idx = 0;
        
        for (chunk_idx, &chunk_offset) in chunk_offsets.iter().enumerate() {
            // 确定这个 chunk 有多少样本
            while stsc_idx + 1 < stsc_entries.len() && stsc_entries[stsc_idx + 1].0 <= chunk_idx + 1 {
                stsc_idx += 1;
            }
            let samples_in_chunk = if stsc_idx < stsc_entries.len() {
                stsc_entries[stsc_idx].1
            } else {
                1
            };
            
            let mut offset_in_chunk = 0;
            for _ in 0..samples_in_chunk {
                if sample_idx >= sample_sizes.len() { break; }
                let size = sample_sizes[sample_idx];
                samples.push((chunk_offset + offset_in_chunk, size));
                offset_in_chunk += size;
                sample_idx += 1;
            }
        }
        
        Ok(samples)
    }
    
    /// 查找 mdat box 的数据起始位置
    fn find_mdat_offset(&self, data: &[u8]) -> Result<usize, String> {
        // mdat 数据直接跟在 box header 后面
        // 返回 0 因为 stco 已经是绝对偏移
        Ok(0)
    }
    
    /// 查找 box 位置
    fn find_box(&self, data: &[u8], box_type: &[u8; 4]) -> Option<usize> {
        for i in 0..data.len().saturating_sub(4) {
            if &data[i..i+4] == box_type {
                return Some(i);
            }
        }
        None
    }
    
    /// 在文件中查找 avcC box
    fn find_avcc_box(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        // 搜索 "avcC" 标记
        let pattern = b"avcC";
        for i in 0..data.len().saturating_sub(4) {
            if &data[i..i+4] == pattern {
                // 找到 avcC，向前读取 box 大小
                if i >= 4 {
                    let box_start = i - 4;
                    let box_size = u32::from_be_bytes([
                        data[box_start], data[box_start+1], 
                        data[box_start+2], data[box_start+3]
                    ]) as usize;
                    
                    if box_start + box_size <= data.len() {
                        // 返回 avcC 内容（跳过 size 和 type）
                        return Ok(data[i+4..box_start+box_size].to_vec());
                    }
                }
            }
        }
        Err("avcC box not found".to_string())
    }
    
    /// 解析 AVCC 数据
    fn parse_avcc(&self, data: &[u8]) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>, usize), String> {
        if data.len() < 7 {
            return Err("AVCC data too short".to_string());
        }
        
        let mut sps_list = Vec::new();
        let mut pps_list = Vec::new();
        
        // configurationVersion = data[0]
        // AVCProfileIndication = data[1]
        // profile_compatibility = data[2]
        // AVCLevelIndication = data[3]
        let nal_length_size = ((data[4] & 0x03) + 1) as usize;
        let num_sps = (data[5] & 0x1F) as usize;
        
        let mut offset = 6;
        
        // 读取 SPS
        for _ in 0..num_sps {
            if offset + 2 > data.len() {
                break;
            }
            let sps_len = u16::from_be_bytes([data[offset], data[offset+1]]) as usize;
            offset += 2;
            
            if offset + sps_len > data.len() {
                break;
            }
            sps_list.push(data[offset..offset+sps_len].to_vec());
            offset += sps_len;
        }
        
        // 读取 PPS 数量
        if offset >= data.len() {
            return Ok((sps_list, pps_list, nal_length_size));
        }
        let num_pps = data[offset] as usize;
        offset += 1;
        
        // 读取 PPS
        for _ in 0..num_pps {
            if offset + 2 > data.len() {
                break;
            }
            let pps_len = u16::from_be_bytes([data[offset], data[offset+1]]) as usize;
            offset += 2;
            
            if offset + pps_len > data.len() {
                break;
            }
            pps_list.push(data[offset..offset+pps_len].to_vec());
            offset += pps_len;
        }
        
        Ok((sps_list, pps_list, nal_length_size))
    }
    
    /// 解码 H.264 样本（指定 NAL 长度大小）
    fn decode_h264_sample_with_nal_size(
        &self, 
        decoder: &mut openh264::decoder::Decoder, 
        data: &[u8],
        nal_length_size: usize,
    ) -> Result<Option<Vec<u8>>, String> {
        let annex_b = self.avcc_to_annex_b_with_size(data, nal_length_size);
        
        match decoder.decode(&annex_b) {
            Ok(Some(yuv)) => {
                let rgba = self.yuv_to_rgba(&yuv);
                Ok(Some(rgba))
            }
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }
    }
    
    /// AVCC 格式转 Annex B 格式（指定 NAL 长度大小）
    fn avcc_to_annex_b_with_size(&self, data: &[u8], nal_length_size: usize) -> Vec<u8> {
        let mut result = Vec::new();
        let mut i = 0;
        
        while i + nal_length_size <= data.len() {
            // 读取 NAL 单元长度
            let nal_len = match nal_length_size {
                1 => data[i] as usize,
                2 => u16::from_be_bytes([data[i], data[i+1]]) as usize,
                3 => ((data[i] as usize) << 16) | ((data[i+1] as usize) << 8) | (data[i+2] as usize),
                4 => u32::from_be_bytes([data[i], data[i+1], data[i+2], data[i+3]]) as usize,
                _ => break,
            };
            i += nal_length_size;
            
            if i + nal_len > data.len() || nal_len == 0 {
                break;
            }
            
            // 添加起始码
            result.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            result.extend_from_slice(&data[i..i + nal_len]);
            i += nal_len;
        }
        
        result
    }
    
    /// YUV 转 RGBA
    fn yuv_to_rgba(&self, yuv: &openh264::decoder::DecodedYUV) -> Vec<u8> {
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
                
                // YUV to RGB conversion (BT.601)
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
                
                let rgba_idx = (row * width + col) * 4;
                rgba[rgba_idx] = r;
                rgba[rgba_idx + 1] = g;
                rgba[rgba_idx + 2] = b;
                rgba[rgba_idx + 3] = 255;
            }
        }
        
        rgba
    }
    
    /// 获取当前帧（基于播放时间同步）
    pub fn get_current_frame(&mut self) -> Option<&VideoFrame> {
        if !self.is_loaded || self.frames.is_empty() {
            return None;
        }
        
        if self.is_playing {
            if let Some(start_time) = self.play_start_time {
                // 计算从播放开始经过的时间
                let elapsed = start_time.elapsed().as_secs_f64();
                // 计算当前应该显示的帧
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
                        // 循环播放
                        self.current_frame = 0;
                        self.play_start_frame = 0;
                        self.play_start_time = Some(Instant::now());
                        // 重新播放音频
                        self.restart_audio();
                    } else {
                        self.current_frame = self.frames.len() - 1;
                        self.is_playing = false;
                        self.stop_audio();
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
            self.last_frame_time = Instant::now();
            
            // 开始播放音频
            self.start_audio();
        }
    }
    
    /// 暂停
    pub fn pause(&mut self) {
        self.is_playing = false;
        self.play_start_time = None;
        self.stop_audio();
    }
    
    /// 开始播放音频
    fn start_audio(&self) {
        if self.muted {
            return;
        }
        
        let audio_data = match &self.audio_data {
            Some(data) => data.clone(),
            None => return,
        };
        
        // 停止之前的音频
        Self::stop_audio_static();
        
        // 计算跳过时间
        let skip_duration = self.frames.get(self.current_frame)
            .map(|f| f.timestamp)
            .unwrap_or(0.0);
        
        // 创建音频输出流
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let cursor = Cursor::new(audio_data);
                if let Ok(source) = Decoder::new(cursor) {
                    if skip_duration > 0.1 {
                        sink.append(source.skip_duration(std::time::Duration::from_secs_f64(skip_duration)));
                    } else {
                        sink.append(source);
                    }
                    
                    sink.play();
                    
                    // 存储到 thread_local
                    AUDIO_STREAM.with(|cell| {
                        *cell.borrow_mut() = Some((stream, sink));
                    });
                    
                    println!("🔊 Audio playback started");
                }
            }
        }
    }
    
    /// 停止音频（静态方法）
    fn stop_audio_static() {
        AUDIO_STREAM.with(|cell| {
            if let Some((_, ref sink)) = *cell.borrow() {
                sink.stop();
            }
            *cell.borrow_mut() = None;
        });
    }
    
    /// 停止音频
    fn stop_audio(&mut self) {
        Self::stop_audio_static();
    }
    
    /// 重新开始音频（用于循环播放）
    fn restart_audio(&self) {
        Self::stop_audio_static();
        
        let audio_data = match &self.audio_data {
            Some(data) => data.clone(),
            None => return,
        };
        
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let cursor = Cursor::new(audio_data);
                if let Ok(source) = Decoder::new(cursor) {
                    sink.append(source);
                    sink.play();
                    
                    AUDIO_STREAM.with(|cell| {
                        *cell.borrow_mut() = Some((stream, sink));
                    });
                }
            }
        }
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
        
        // 默认视频大小 300x225 (4:3)
        let default_width = 300.0;
        let default_height = 225.0;
        
        if ts.size.width == Dimension::Auto {
            ts.size.width = length(default_width * sf);
        }
        if ts.size.height == Dimension::Auto {
            ts.size.height = length(default_height * sf);
        }
        
        // 视频背景
        ns.background_color = Some(Color::BLACK);
        ns.border_radius = 4.0 * sf;
        
        let tn = ctx.taffy.new_leaf(ts).unwrap();
        
        // 预加载视频
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
        x: f32, 
        y: f32, 
        w: f32, 
        h: f32, 
        sf: f32
    ) {
        let style = &node.style;
        let radius = style.border_radius;
        let src = &node.text;
        
        // 绘制背景
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
        
        // 尝试获取视频帧
        if !src.is_empty() {
            if let Some((frame_data, frame_w, frame_h)) = get_video_frame(src) {
                // 绘制视频帧
                canvas.draw_image(
                    &frame_data,
                    frame_w,
                    frame_h,
                    x, y, w, h,
                    "aspectFit",
                    radius,
                );
                
                // 绘制控制条
                Self::draw_controls(canvas, text_renderer, src, x, y, w, h, sf);
                return;
            }
        }
        
        // 如果没有视频帧，绘制占位符
        Self::draw_placeholder(canvas, x, y, w, h, sf);
    }
    
    /// 绘制视频占位符
    fn draw_placeholder(canvas: &mut Canvas, x: f32, y: f32, w: f32, h: f32, sf: f32) {
        let cx = x + w / 2.0;
        let cy = y + h / 2.0;
        let btn_size = 50.0 * sf;
        
        // 半透明圆形背景
        let bg_paint = Paint::new()
            .with_color(Color::new(0, 0, 0, 128))
            .with_style(PaintStyle::Fill)
            .with_anti_alias(true);
        canvas.draw_circle(cx, cy, btn_size / 2.0, &bg_paint);
        
        // 播放三角形
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
    
    /// 绘制控制条
    fn draw_controls(
        canvas: &mut Canvas, 
        text_renderer: Option<&TextRenderer>,
        src: &str,
        x: f32, y: f32, w: f32, h: f32, 
        sf: f32
    ) {
        let bar_height = 36.0 * sf;
        let bar_y = y + h - bar_height;
        
        // 半透明背景
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
            // 暂停图标
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
            // 播放图标
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
        
        // 进度条
        if let Some((current, duration)) = get_video_progress(src) {
            let progress_x = btn_x + btn_size + 12.0 * sf;
            let progress_w = w - progress_x - 80.0 * sf - x;
            let progress_h = 4.0 * sf;
            let progress_y = bar_y + (bar_height - progress_h) / 2.0;
            
            // 进度条背景
            let track_paint = Paint::new()
                .with_color(Color::new(255, 255, 255, 80))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w, progress_h), &track_paint);
            
            // 进度条填充
            let progress = if duration > 0.0 { current / duration } else { 0.0 };
            let fill_paint = Paint::new()
                .with_color(Color::from_hex(0x07C160))
                .with_style(PaintStyle::Fill);
            canvas.draw_rect(&GeoRect::new(progress_x, progress_y, progress_w * progress as f32, progress_h), &fill_paint);
            
            // 时间文本
            if let Some(tr) = text_renderer {
                let time_text = format!("{} / {}", 
                    Self::format_time(current), 
                    Self::format_time(duration)
                );
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
