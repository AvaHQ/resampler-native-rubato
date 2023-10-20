use log::{debug, error};
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::Read;
use std::io::{BufWriter, Write};

const BYTE_PER_SAMPLE: usize = 8;

// F64 is required, panic if f32
pub fn buffer_to_vecs<R: Read>(input_buffer: &mut R, channels: usize) -> Vec<Vec<f64>> {
  let mut buffer = vec![0u8; BYTE_PER_SAMPLE];
  let mut wfs = Vec::with_capacity(channels);
  for _chan in 0..channels {
    wfs.push(Vec::new());
  }
  'outer: loop {
    for wf in wfs.iter_mut() {
      let bytes_read = input_buffer.read(&mut buffer).unwrap();
      if bytes_read == 0 {
        break 'outer;
      }
      let value = f64::from_le_bytes(buffer.as_slice().try_into().unwrap()); //Little endian
      wf.push(value);
    }
  }
  wfs
}

pub fn skip_frames(
  frames: Vec<Vec<f64>>,
  frames_to_skip: usize,
  frames_to_write: usize,
) -> Result<Vec<f64>, String> {
  let mut collected_data: Vec<f64> = Vec::new();
  let channels = frames.len();
  let end = frames_to_skip + frames_to_write;
  if end > frames[0].len() {
    return Err(format!(
      "End frames_to_skip + frames_to_write {} is above the length of frames which are {}",
      end,
      frames.len()
    ));
  }
  for frame_to_skip in frames_to_skip..end {
    for frame in frames.iter().take(channels) {
      let value64 = frame[frame_to_skip];
      collected_data.extend_from_slice(&[value64]);
    }
  }
  Ok(collected_data)
}

pub fn append_frames(buffers: &mut [Vec<f64>], additional: &[Vec<f64>], nbr_frames: usize) {
  buffers
    .iter_mut()
    .zip(additional.iter())
    .for_each(|(b, a)| b.extend_from_slice(&a[..nbr_frames]));
}

/// Helper to write all frames to a file
pub fn write_frames_to_disk(frames: Vec<u8>, output: String) {
  let file = File::create(output).expect("Cannot create output file");
  let mut file_out_disk = BufWriter::new(file);

  if let Err(err) = file_out_disk.write_all(&frames) {
    error!("Cannot send data to file : {:?}", err);
  }

  if let Err(err) = file_out_disk.flush() {
    error!("Cannot clear tmp : {:?}", err);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /**
   * ? buffer_to_vecs Unit Tests
   */

  #[test]
  fn test_buffer_to_vecs_single_channel() {
    let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 1;
    let result = buffer_to_vecs(&mut input_buffer, channels);

    // mono so all inside same deep vec
    let expected_result: Vec<Vec<f64>> = vec![vec![
      f64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
      f64::from_le_bytes([9, 10, 11, 12, 13, 14, 15, 16]),
    ]];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_buffer_to_vecs_multiple_channels() {
    let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 2;
    let result = buffer_to_vecs(&mut input_buffer, channels);

    // stereo so vec of vec for channels
    let expected_result: Vec<Vec<f64>> = vec![
      vec![f64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8])],
      vec![f64::from_le_bytes([9, 10, 11, 12, 13, 14, 15, 16])],
    ];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_buffer_to_vecs_empty_input() {
    let data: &[u8] = &[];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 1;
    let result = buffer_to_vecs(&mut input_buffer, channels);

    let expected_result: Vec<Vec<f64>> = vec![vec![]];
    assert_eq!(result, expected_result);
  }

  /**
   * ? skip_frames Unit Tests
   */

  #[test]
  fn test_skip_frames_should_return_an_error() {
    // because will be out of range of clusion
    let frames: Vec<Vec<f64>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 4;
    let frames_to_write = 1;

    let result = skip_frames(frames, frames_to_skip, frames_to_write);

    if let Err(err) = result {
      assert!(err.to_string().contains("frames_to_write 5 is above"));
    }
  }

  #[test]
  fn test_skip_frames_no_frames_to_write() {
    let frames: Vec<Vec<f64>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 1;
    let frames_to_write = 0;

    let result = skip_frames(frames, frames_to_skip, frames_to_write).unwrap();

    // Expected result: Empty vector
    let expected_result: Vec<f64> = vec![];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_skip_frames_skip_all_frames() {
    let frames: Vec<Vec<f64>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 3;
    let frames_to_write = 0;

    let result = skip_frames(frames, frames_to_skip, frames_to_write).unwrap();

    // Expected result: Empty vector
    let expected_result: Vec<f64> = vec![];
    assert_eq!(result, expected_result);
  }
}
