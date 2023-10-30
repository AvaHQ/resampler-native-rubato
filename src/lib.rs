extern crate env_logger;
extern crate napi_derive;
extern crate num_traits;
extern crate rubato;

mod helpers;

use log::debug;
use num_traits::FromPrimitive;
use rubato::{implement_resampler, FastFixedIn, PolynomialDegree};

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::time::Instant;
use std::vec;

use napi::bindgen_prelude::*;
use napi::JsUndefined;
use napi_derive::napi;

use crate::helpers::{
  append_frames, f32_buffer_to_vecs, i16_buffer_to_vecs, skip_frames, write_frames_to_disk,
};

implement_resampler!(SliceResampler, &[&[T]], &mut [Vec<T>]);

use napi::module_init;

#[module_init]
fn init() {
  env_logger::init();
}

/**
 * N-API.RS exported functions via macro
 */
#[napi(object)]
pub struct ArgsAudioToReSample {
  pub sample_rate_input: u16,
  pub sample_rate_output: u16,
  pub channels: u8,
}

#[napi]
pub enum DataType {
  I16,
  F32,
}

#[napi(object)]
pub struct ArgsAudioFile {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_raw_path: String,
  pub output_path: String,
  pub type_of_bin_data: DataType,
}

#[napi]
pub fn re_sample_audio_file(args: ArgsAudioFile) {
  let ArgsAudioFile {
    input_raw_path,
    output_path,
    args_audio_to_re_sample,
    type_of_bin_data,
  } = args;
  let ArgsAudioToReSample {
    channels,
    sample_rate_input,
    sample_rate_output,
  } = args_audio_to_re_sample;
  let file_in_disk = File::open(input_raw_path).expect("Can't open file");
  let mut file_in_reader = BufReader::new(file_in_disk);

  // Depending of sub-data-type we can use i16 or f32
  let indata: Vec<Vec<f32>> = match type_of_bin_data {
    DataType::I16 => f32_buffer_to_vecs(&mut file_in_reader, 2),
    DataType::F32 => i16_buffer_to_vecs(&mut file_in_reader, 2),
  };

  let start = Instant::now();
  let res = re_sample_audio_buffer(
    indata,
    sample_rate_input,
    sample_rate_output,
    channels,
    channels,
  );

  let mut result: Vec<u8> = Vec::new();
  result.extend(res.iter().flat_map(|&f| f.to_le_bytes()));
  write_frames_to_disk(result, output_path);
  debug!("Time to convert the file was {:?}", start.elapsed());
  JsUndefined::value_type();
}
#[napi(object)]
pub struct ArgsAudioBuffer {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_buffer: Buffer,
}

#[napi]
pub fn re_sample_buffers(args: ArgsAudioBuffer) -> Buffer {
  let ArgsAudioBuffer {
    args_audio_to_re_sample,
    input_buffer,
  } = args;
  let ArgsAudioToReSample {
    channels,
    sample_rate_input,
    sample_rate_output,
  } = args_audio_to_re_sample;
  let buffer_conversion_time = Instant::now();
  debug!(
    "Before buffer_i16_to_vecs length is {}",
    &input_buffer.len()
  );
  let mut read_buffer = Box::new(Cursor::new(&input_buffer));
  let data = f32_buffer_to_vecs(&mut read_buffer, channels as usize);
  debug!("After buffer_i16_to_vecs length is {}", &data[0].len());
  debug!(
    "It took {:?} to convert {} buffer elements vec to vec<vec<f32>> with [0] contains {} and [1] {}",
    buffer_conversion_time.elapsed(),
    input_buffer.len(),
    data[0].len(),
    data[1].len()
  );

  let output_data = re_sample_audio_buffer(
    data,
    sample_rate_input,
    sample_rate_output,
    channels,
    channels,
  );

  let mut result: Vec<u8> = Vec::new();
  result.extend(output_data.iter().flat_map(|&f| f.to_le_bytes()));
  result.into()
}

#[napi(object)]
pub struct ArgsAudioInt16Buffer {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_int16_buffer: Buffer,
}

#[napi]
pub fn re_sample_int_16_buffer(args: ArgsAudioInt16Buffer) -> Buffer {
  let ArgsAudioInt16Buffer {
    args_audio_to_re_sample,
    input_int16_buffer,
  } = args;

  let ArgsAudioToReSample {
    channels,
    sample_rate_input,
    sample_rate_output,
  } = args_audio_to_re_sample;
  let convert_i16_time = Instant::now();
  let mut read_buffer = Box::new(Cursor::new(&input_int16_buffer));
  let i16_data = i16_buffer_to_vecs(&mut read_buffer, 2);
  debug!(
    "It took {:?} to convert {} i16 elements vec to vec<vec<f32>> with [0] contains {} and [1] {}",
    convert_i16_time.elapsed(),
    input_int16_buffer.len(),
    i16_data[0].len(),
    i16_data[1].len()
  );

  let output_data = re_sample_audio_buffer(
    i16_data,
    sample_rate_input,
    sample_rate_output,
    channels,
    channels,
  );

  let convert_i16_back_time = Instant::now();

  // Issue is before this !
  let i16_ouput: Vec<i16> = output_data
    .iter()
    .map(|&f32_value| i16::from_f32(f32_value * f32::from_i16(i16::MAX).unwrap()).unwrap())
    .collect();

  debug!(
    "It took {:?} to convert i16 vec to vec<vec<f32>>",
    convert_i16_back_time.elapsed()
  );

  let mut buffer: Vec<u8> = Vec::new();
  buffer.extend(i16_ouput.iter().flat_map(|&f| f.to_le_bytes()));

  buffer.into()
}

/**
 * This is the Rust main smart ,function, use all pure function inside
 * Main logic is here
 */
fn re_sample_audio_buffer(
  buffer: Vec<Vec<f32>>,
  input_sample_rate: u16,
  output_sample_rate: u16,
  input_channels: u8,
  output_channels: u8,
) -> Vec<f32> {
  let fs_in = input_sample_rate as usize;
  let channels = input_channels as usize;
  let nbr_input_frames = buffer[0].len(); // ? because for stereo
  let duration_total = Instant::now();

  let fs_out = output_sample_rate;
  debug!("Sample {} for output {}", &fs_in, &fs_out);

  // Create buffer for storing output
  let mut outdata =
    vec![
      Vec::with_capacity(2 * (nbr_input_frames as f32 * fs_out as f32 / fs_in as f32) as usize);
      channels
    ];

  let f_ratio = fs_out as f64 / fs_in as f64;

  let mut resampler = FastFixedIn::<f32>::new(
    f_ratio,
    1.1,
    PolynomialDegree::Septic,
    1024,
    output_channels as usize,
  )
  .unwrap();

  // Prepare
  let mut input_frames_next = resampler.input_frames_next();
  let resampler_delay = resampler.output_delay();
  let mut outbuffer = vec![vec![0.0f32; resampler.output_frames_max()]; channels];
  let mut indata_slices: Vec<&[f32]> = buffer.iter().map(|v| &v[..]).collect();

  // Process all full chunks
  while indata_slices[0].len() >= input_frames_next {
    let (nbr_in, nbr_out) = resampler
      .process_into_buffer(&indata_slices, &mut outbuffer, None)
      .unwrap();
    for chan in indata_slices.iter_mut() {
      *chan = &chan[nbr_in..];
    }
    append_frames(&mut outdata, &outbuffer, nbr_out);
    input_frames_next = resampler.input_frames_next();
  }

  // Process a partial chunk with the last frames.
  if !indata_slices[0].is_empty() {
    let (_nbr_in, nbr_out) = resampler
      .process_partial_into_buffer(Some(&indata_slices), &mut outbuffer, None)
      .unwrap();
    append_frames(&mut outdata, &outbuffer, nbr_out);
  }

  let nbr_output_frames = (nbr_input_frames as f64 * fs_out as f64 / fs_in as f64) as usize;

  let duration_total_time = duration_total.elapsed();
  debug!("Resampling buffer took: {:?}", duration_total_time);

  skip_frames(outdata, resampler_delay, nbr_output_frames).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_re_sample_audio_buffer_single_channel() {
    let buffer = vec![vec![0.0, 1.0, 2.0, 3.0, 4.0]];
    let input_sample_rate = 44100;
    let output_sample_rate = 48000;
    let input_channels = 1;
    let output_channels = 1;

    let result = re_sample_audio_buffer(
      buffer,
      input_sample_rate,
      output_sample_rate,
      input_channels,
      output_channels,
    );

    assert_eq!(result.len(), 5); // I do not know if those test are revelant, any there for no regression
  }
  #[test]
  fn test_re_sample_audio_buffer_stereo() {
    let buffer = vec![vec![0.0, 1.0, 2.0, 3.0, 4.0], vec![5.0, 6.0, 7.0, 8.0, 9.0]];
    let input_sample_rate = 44100;
    let output_sample_rate = 48000;
    let input_channels = 2;
    let output_channels = 2;

    let result = re_sample_audio_buffer(
      buffer,
      input_sample_rate,
      output_sample_rate,
      input_channels,
      output_channels,
    );

    assert_eq!(result.len(), 10);
  }
}
