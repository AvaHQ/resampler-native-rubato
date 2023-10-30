extern crate env_logger;
extern crate num_traits;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error};
use num_traits::FromPrimitive;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::Read;
use std::io::{BufWriter, Write};

const BYTE_PER_SAMPLE: usize = 4;
static LOGGER_INITIALIZED: std::sync::Once = std::sync::Once::new();

/**
 Reads data from a Read trait and converts it into a vector of vectors containing 64-bit floating-point numbers (f64).

 # Arguments

 * `input_reader` - A mutable reference to a type implementing the Read trait, such as a file or a buffer. The bytes of this buffer always assume to represent u8 number of little endian
 * `channels` - The number of channels in the resulting vector of vectors.

 The samples are memory contigus, so we have two samples for two channels.
  The byte order of the data is: time step, channel, sample byte.
  That is, for an example of 2 time steps, 2 channels, and (as always) 2 bytes per sample

 # Returns

 Returns a vector of vectors, where each inner vector represents a channel of audio data as 64-bit floating-point numbers (f64).

 # Example

 ```
 use std::fs::File;
 use std::io::Read;
 use my_audio_library::buffer_to_vecs;

 let mut file = File::open("audio.bin").expect("Failed to open file");
 let channels = 2;

 let result = buffer_to_vecs(&mut file, channels);

 // You can now process the resulting audio data.
 ```
*/
pub fn buffer_to_vecs<R: Read>(input_reader: &mut R, channels: usize) -> Vec<Vec<f32>> {
  let mut buffer = vec![0u8; BYTE_PER_SAMPLE];
  let mut audio_data = vec![Vec::new(); channels];
  'conversion_loop: loop {
    // dispatch the data between channels
    for audio_single_channel in audio_data.iter_mut() {
      let bytes_read = input_reader.read(&mut buffer).unwrap();
      if bytes_read == 0 {
        break 'conversion_loop;
      }
      let value = match buffer.as_slice().try_into() {
        Ok(bytes) => f32::from_le_bytes(bytes),
        Err(error) => {
          error!("Error of conversion to f32 {}", error.to_string());
          0.0
        }
      };
      audio_single_channel.push(value);
    }
  }
  audio_data
}

/**
 Converts a vector of signed 16-bit integers (i16) into a vector of vectors containing 64-bit floating-point numbers (f64).

 # Arguments

 * `input_reader` - A buffer of signed 16-bit integers to be converted.
 * `channels` - The number of channels in the resulting vector of vectors.

The samples are memory contigus, so we have two samples for two channels.
The byte order of the data is: time step, channel, sample byte.
That is, for an example of 2 time steps, 2 channels, and (as always) 2 bytes per sample

 # Returns

 Returns a vector of vectors, where each inner vector represents a channel of audio data as 64-bit floating-point numbers (f64).

 # Example

 ```
 use std::fs::File;
 use std::io::Read;
 use my_audio_library::i16_buffer_to_vecs;

 let mut file = File::open("audio.bin").expect("Failed to open file");
 let channels = 2;

 let result = i16_buffer_to_vecs(&mut file, channels);

 assert_eq!(result.len(), channels);
 assert_eq!(result[0], vec![123.0, 456.0, 789.0, -321.0]);
 assert_eq!(result[1], vec![654.0, -987.0]);
 ```
*/
pub fn i16_buffer_to_vecs<R: Read>(input_reader: &mut R, channels: usize) -> Vec<Vec<f32>> {
  let mut audio_data = Vec::with_capacity(channels);
  for _chan in 0..channels {
    audio_data.push(Vec::new());
  }
  'outer: loop {
    // dispatch the data between channels
    for audio_single_channel in audio_data.iter_mut() {
      match input_reader.read_i16::<LittleEndian>() {
        Ok(value_i16) => {
          // let bytes = value_i16.to_le_bytes();
          let value_f64 = f32::from_i16(value_i16).unwrap() / f32::from_i16(i16::MAX).unwrap();
          // if (value_i16 != 0) {
          //   debug!("value_i16 {:?} value_f64 {:?}", value_i16, value_f64)
          // }
          audio_single_channel.push(value_f64);
        }
        Err(err) => {
          break 'outer; // end of loop err happen when oef for eg
        }
      }
    }
  }
  audio_data
}

/** Skips a specified number of frames in a multi-channel audio signal and collects a certain number of subsequent frames.

 # Arguments

 * `frames` - A vector of vectors, where each inner vector represents a channel of audio data.
 * `frames_to_skip` - The number of frames to skip at the beginning of each channel.
 * `frames_to_write` - The number of frames to collect for each channel after skipping frames.

 # Returns

 Returns a `Result` where `Ok` contains a vector of collected audio data as `f64` values, and `Err` contains an error message if frames_to_skip + frames_to_write exceeds the length of the input frames for any channel.

 # Example

 ```
 use my_audio_library::skip_frames;

 let frames: Vec<Vec<f64>> = vec![
     vec![1.0, 2.0, 3.0, 4.0],
     vec![5.0, 6.0, 7.0, 8.0],
 ];

 let frames_to_skip = 1;
 let frames_to_write = 2;

 let result = skip_frames(frames, frames_to_skip, frames_to_write);

 assert!(result.is_ok());
 ```

 # Errors

 Returns an error if `frames_to_skip + frames_to_write` exceeds the length of frames for any channel.
**/

pub fn skip_frames(
  frames: Vec<Vec<f32>>,
  frames_to_skip: usize,
  frames_to_write: usize,
) -> Result<Vec<f32>, String> {
  let mut collected_data: Vec<f32> = Vec::new();
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

/**
 Appends a specified number of frames from one set of audio buffers to another set of audio buffers.

 # Arguments

 * `buffers` - A mutable reference to a slice of vectors representing audio buffers.
 * `additional` - A slice of vectors containing additional audio frames to append.
 * `nbr_frames` - The number of frames to append from the `additional` vector to each buffer in `buffers`.

 # Example

 ```
 use my_audio_library::append_frames;

 let mut audio_buffers: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
 let additional_frames: Vec<Vec<f64>> = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
 let num_frames_to_append = 1;

 append_frames(&mut audio_buffers, &additional_frames, num_frames_to_append);

 // Now, audio_buffers contains the appended audio data.
 ```
*/
pub fn append_frames(buffers: &mut [Vec<f32>], additional: &[Vec<f32>], nbr_frames: usize) {
  buffers
    .iter_mut()
    .zip(additional.iter())
    .for_each(|(b, a)| b.extend_from_slice(&a[..nbr_frames]));
}

/**
Write a vector of bytes to a file on disk.

This function takes a vector of bytes (`frames`) and a file path (`output`), and writes the
bytes to the specified file. It creates the file if it doesn't exist and overwrites it if it
does. The function also handles errors related to file creation and writing.

# Arguments

* `frames` - A vector of bytes to be written to the file.
* `output` - The file path where the bytes will be written.

# Errors

If an error occurs during file creation or writing, this function will log the error using the
`error!` macro from a logging framework (not shown here). The error message will contain
details about the specific error encountered.

# Examples

```rust
# use std::fs;
# use tempfile::tempdir;

# fn main() {
#     let temp_dir = tempdir().expect("Failed to create temporary directory");
#     let output_file = temp_dir.path().join("output.bin");
let frames = vec![0, 1, 2, 3, 4];
let output_path = output_file.to_str().expect("Invalid path").to_string();

write_frames_to_disk(frames, output_path);

// You can now assert the contents of the file or handle any errors gracefully in tests.
# }
```
*/
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

/**
 * Singleton of logger because it cannot be instanciated more than once
 */
pub fn initialize_logger() {
  LOGGER_INITIALIZED.call_once(|| {
    env_logger::init();
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  /**
   * ? i16_vec_to_vecsc Unit Tests
   */
  #[test]
  fn test_i16_vec_to_vecs_stereo() {
    initialize_logger();
    let i16_values: Vec<i16> = vec![123, 456, 789, -321, 654, -987];
    let u8_values: &[u8] = unsafe {
      std::slice::from_raw_parts(
        i16_values.as_ptr() as *const u8,
        i16_values.len() * std::mem::size_of::<i16>(),
      )
    };

    let mut reader_data = Cursor::new(u8_values);
    let channels = 2;

    let result = i16_buffer_to_vecs(&mut reader_data, channels);

    assert_eq!(result.len(), channels);

    assert_eq!(result[0], vec![0.0037537767, 0.024079105, 0.019959105]); // representing i16 abov number but to f32 divide by max of i16
    assert_eq!(result[1], vec![0.01391644, -0.0097964415, -0.03012177]);
  }
  #[test]
  fn test_i16_vec_to_vecs_mono() {
    initialize_logger();
    let i16_values: Vec<i16> = vec![123, 456, 789, -321, 654, -987];
    let u8_values: &[u8] = unsafe {
      std::slice::from_raw_parts(
        i16_values.as_ptr() as *const u8,
        i16_values.len() * std::mem::size_of::<i16>(),
      )
    };
    let mut reader_data = Cursor::new(u8_values);
    let channels = 1;

    let result = i16_buffer_to_vecs(&mut reader_data, channels);

    assert_eq!(result.len(), channels);
    // Should be in range [-1.0;1.0] for audio
    assert_eq!(
      result[0],
      vec![
        0.0037537767,
        0.01391644,
        0.024079105,
        -0.0097964415,
        0.019959105,
        -0.03012177
      ]
    );
  }

  /**
   * ? buffer_to_vecs Unit Tests
   */
  #[test]
  fn test_buffer_to_vecs_single_channel() {
    initialize_logger();
    let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 1;
    let result = buffer_to_vecs(&mut input_buffer, channels);
    // mono so all inside same deep vec
    let expected_result: Vec<Vec<f32>> = vec![vec![
      f32::from_le_bytes([1, 2, 3, 4]),
      f32::from_le_bytes([5, 6, 7, 8]),
      f32::from_le_bytes([9, 10, 11, 12]),
      f32::from_le_bytes([13, 14, 15, 16]),
    ]];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_buffer_to_vecs_multiple_channels() {
    initialize_logger();
    let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 2;
    let result = buffer_to_vecs(&mut input_buffer, channels);

    // stereo so vec of vec for channels
    let expected_result: Vec<Vec<f32>> = vec![
      vec![f32::from_le_bytes([1, 2, 3, 4])],
      vec![f32::from_le_bytes([5, 6, 7, 8])],
      vec![f32::from_le_bytes([9, 10, 11, 12])],
      vec![f32::from_le_bytes([13, 14, 15, 16])],
    ];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_buffer_to_vecs_empty_input() {
    initialize_logger();
    let data: &[u8] = &[];
    let mut input_buffer = std::io::Cursor::new(data);

    let channels = 1;
    let result = buffer_to_vecs(&mut input_buffer, channels);

    let expected_result: Vec<Vec<f32>> = vec![vec![]];
    assert_eq!(result, expected_result);
  }

  /**
   * ? skip_frames Unit Tests
   */
  #[test]
  fn test_skip_frames_should_return_an_error() {
    initialize_logger();
    // because will be out of range of clusion
    let frames: Vec<Vec<f32>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 4;
    let frames_to_write = 1;

    let result = skip_frames(frames, frames_to_skip, frames_to_write);

    if let Err(err) = result {
      assert!(err.to_string().contains("frames_to_write 5 is above"));
    }
  }

  #[test]
  fn test_skip_frames_no_frames_to_write() {
    initialize_logger();
    let frames: Vec<Vec<f32>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 1;
    let frames_to_write = 0;

    let result = skip_frames(frames, frames_to_skip, frames_to_write).unwrap();

    // Expected result: Empty vector
    let expected_result: Vec<f32> = vec![];
    assert_eq!(result, expected_result);
  }

  #[test]
  fn test_skip_frames_skip_all_frames() {
    initialize_logger();
    let frames: Vec<Vec<f32>> = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
    let frames_to_skip = 3;
    let frames_to_write = 0;

    let result = skip_frames(frames, frames_to_skip, frames_to_write).unwrap();

    // Expected result: Empty vector
    let expected_result: Vec<f32> = vec![];
    assert_eq!(result, expected_result);
  }

  /**
   * ? append_frames Unit Tests
   */
  #[test]
  fn test_append_frames() {
    initialize_logger();
    let mut audio_buffers: Vec<Vec<f32>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let additional_frames: Vec<Vec<f32>> = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
    let num_frames_to_append = 1;

    append_frames(&mut audio_buffers, &additional_frames, num_frames_to_append);

    assert_eq!(audio_buffers[0], vec![1.0, 2.0, 5.0]);
    assert_eq!(audio_buffers[1], vec![3.0, 4.0, 7.0]);
  }
  use std::{fs, io::Cursor};
  use tempfile::tempdir;
  #[test]
  fn test_write_frames_to_disk() {
    initialize_logger();
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let output_file = temp_dir.path().join("output.bin");
    let output_path = output_file.to_str().expect("Invalid path").to_string();
    let frames = vec![0, 1, 2, 3, 4];

    write_frames_to_disk(frames.clone(), output_path);

    // Read the file and check its contents
    let file_contents = fs::read(output_file).expect("Failed to read file");
    assert_eq!(file_contents, frames);
  }
}
