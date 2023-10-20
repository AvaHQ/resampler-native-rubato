#[macro_use]
extern crate napi_derive;
extern crate env_logger;
extern crate rubato;
mod helpers;

use log::debug;
use rubato::{implement_resampler, FastFixedIn, PolynomialDegree};

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::time::Instant;
use std::vec;

use napi::bindgen_prelude::*;
use napi::JsUndefined;
use napi_derive::napi;

use crate::helpers::{append_frames, buffer_to_vecs, skip_frames, write_frames_to_disk};

implement_resampler!(SliceResampler, &[&[T]], &mut [Vec<T>]);

/**
 * N-API.RS exported functions via macro
 */
#[napi(object)]
pub struct ArgsAudioToReSample {
  pub sample_rate_input: u16,
  pub sample_rate_output: u16,
  pub channels: u8,
}

#[napi(object)]
pub struct ArgsAudioFile {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_raw_path: String,
  pub output_path: String,
}

#[napi]
pub fn re_sample_audio_file(args: ArgsAudioFile) {
  // call the buffer resampler fn here + write to file
  let ArgsAudioFile {
    input_raw_path,
    output_path,
    args_audio_to_re_sample,
  } = args;
  let ArgsAudioToReSample {
    channels,
    sample_rate_input,
    sample_rate_output,
  } = args_audio_to_re_sample;

  let file_in_disk = File::open(input_raw_path).expect("Can't open file");
  let mut file_in_reader = BufReader::new(file_in_disk);
  debug!("Data inside buffer {}", file_in_reader.capacity());

  let indata = buffer_to_vecs(&mut file_in_reader, 2);
  debug!("re_sample_audio_file indata lenght {}", indata.len());

  //re_sample_audio_buffer
  let start = Instant::now();
  let res = re_sample_audio_buffer(
    indata,
    sample_rate_input,
    sample_rate_output,
    channels,
    channels,
  );
  debug!("Time for convert the file is {:?}", start.elapsed());

  let mut result: Vec<u8> = Vec::new();
  result.extend(res.iter().flat_map(|&f| f.to_le_bytes()));
  write_frames_to_disk(result, output_path);
  JsUndefined::value_type();
}

#[napi(object)]
pub struct ArgsAudioBuffer {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_buffer: Buffer,
}
pub struct ArgsAudioInt16Array {
  pub args_audio_to_re_sample: ArgsAudioToReSample,
  pub input_buffer: Int16Array,
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
  let input_slice = input_buffer.to_vec();
  let mut read_buffer = Box::new(Cursor::new(&input_slice));
  let data = buffer_to_vecs(&mut read_buffer, channels as usize);

  debug!(
    " Size of input_slice {} and {}",
    input_slice.len(),
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

#[napi]
pub fn re_sample_int16Array(input_buffer: Int16Array) -> Int16Array {
  let input_slice = input_buffer.to_vec();
  let mut result = Vec::new();

  for i in (0..input_slice.len()).step_by(2) {
    if i + 1 < input_slice.len() {
      let i16_value = i16::from_le(input_slice[i]);
      let f64_value = f64::from(i16_value);
      result.push(vec![f64_value]);
    }
  }

  let output_data = re_sample_audio_buffer(result, 44100, 16000, 2, 2);

  let res: Vec<i16> = output_data
    .iter()
    .map(|&f64_value| {
      let i64_value = f64_value.to_bits() as i64;
      if i64_value > i16::MAX as i64 {
        i16::MAX
      } else if i64_value < i16::MIN as i64 {
        i16::MIN
      } else {
        i64_value as i16
      }
    })
    .collect();

  let res2 = Int16Array::new(res);

  res2.into()
}

/**
 * Rust helpers functions
 */

fn re_sample_audio_buffer(
  buffer: Vec<Vec<f64>>,
  input_sample_rate: u16,
  output_sample_rate: u16,
  input_channels: u8,
  output_channels: u8,
) -> Vec<f64> {
  debug!("buffer size {}", buffer.len());

  let fs_in = input_sample_rate as usize;
  let channels = input_channels as usize;
  let nbr_input_frames = buffer[0].len(); // ? because for stereo
  let duration_total = Instant::now();

  let fs_out = output_sample_rate;
  debug!("Sample {} for output {}", &fs_in, &fs_out);

  // Create buffer for storing output
  let mut outdata =
    vec![
      Vec::with_capacity(2 * (nbr_input_frames as f64 * fs_out as f64 / fs_in as f64) as usize);
      channels
    ];

  let f_ratio = fs_out as f64 / fs_in as f64;

  let mut resampler = FastFixedIn::<f64>::new(
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
  let mut outbuffer = vec![vec![0.0f64; resampler.output_frames_max()]; channels];
  let mut indata_slices: Vec<&[f64]> = buffer.iter().map(|v| &v[..]).collect();

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
  debug!("Resampling file took: {:?}", duration_total_time);

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
