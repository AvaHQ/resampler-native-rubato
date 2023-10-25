use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use log::{debug, error};
use napi::bindgen_prelude::Buffer;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::Read;
use std::io::{BufWriter, Seek, Write};

const BYTE_PER_SAMPLE: usize = 8;
/**
 Reads data from a Read trait and converts it into a vector of vectors containing 64-bit floating-point numbers (f64).

 # Arguments

 * `input_buffer` - A mutable reference to a type implementing the Read trait, such as a file or a buffer. The bytes of this buffer always assume to represent i16 number of little endian
 * `channels` - The number of channels in the resulting vector of vectors.

 The samples are memory contigus, so we have two samples for two channels.
  The byte order of the data is: time step, channel, sample byte.
  That is, for an example of 2 time steps, 2 channels, and (as always) 2 bytes per sample, the memory content of input_buffer is:

  T1 C1 S1
  T1 C1 S2
  T1 C2 S1
  T1 C2 S2
  T2 C1 S1
  T2 C1 S2
  T2 C2 S1
  T2 C2 S2

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
pub fn buffer_to_vecs<R: Read>(inbuffer: &mut R, channels: usize) -> Vec<Vec<f64>> {
  let mut wfs = Vec::with_capacity(channels);
  for _chan in 0..channels {
    wfs.push(Vec::new());
  }
  'outer: loop {
    for wf in wfs.iter_mut() {
      match inbuffer.read_i16::<LittleEndian>() {
        Ok(value_i16) => {
          let value_f64 = f64::from(value_i16);
          wf.push(value_f64);
        }
        Err(err) => {
          break 'outer;
        }
      }
    }
  }
  wfs
}

/**
 Converts a vector of signed 16-bit integers (i16) into a vector of vectors containing 64-bit floating-point numbers (f64).

 # Arguments

 * `input_data` - A vector of signed 16-bit integers to be converted.
 * `channels` - The number of channels in the resulting vector of vectors.

 # Returns

 Returns a vector of vectors, where each inner vector represents a channel of audio data as 64-bit floating-point numbers (f64).

 # Example

 ```
 use my_audio_library::i16_vec_to_vecs;

 let input_data: Vec<i16> = vec![123, 456, 789, -321, 654, -987];
 let channels = 2;

 let result = i16_vec_to_vecs(input_data, channels);

 assert_eq!(result.len(), channels);
 assert_eq!(result[0], vec![123.0, 456.0, 789.0, -321.0]);
 assert_eq!(result[1], vec![654.0, -987.0]);
 ```
*/
pub fn i16_vec_to_vecs(input_data: &[i16], channels: usize) -> Vec<Vec<f64>> {
  let mut wfs = vec![Vec::with_capacity(input_data.len() / channels); channels];

  for (i, &i16_value) in input_data.iter().enumerate() {
    let f64_value = i16_value as f64;
    let channel_index = i % channels;
    wfs[channel_index].push(f64_value);
  }

  wfs
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
pub fn append_frames(buffers: &mut [Vec<f64>], additional: &[Vec<f64>], nbr_frames: usize) {
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

#[cfg(test)]
mod tests {
  use super::*;

  /**
   * ? i16_vec_to_vecsc Unit Tests
   */

  #[test]
  fn test_i16_vec_to_vecs_stereo() {
    // Créez un exemple de données d'entrée
    let input_data: Vec<i16> = vec![123, 456, 789, -321, 654, -987];
    let channels = 2;

    // Appelez la fonction pour obtenir le résultat
    let result = i16_vec_to_vecs(&input_data, channels);

    // Vérifiez que le résultat a le nombre attendu de canaux
    assert_eq!(result.len(), channels);

    // Vérifiez le contenu du résultat
    assert_eq!(result[0], vec![123.0, 789.0, 654.0]);
    assert_eq!(result[1], vec![456.0, -321.0, -987.0]);
  }
  #[test]
  fn test_i16_vec_to_vecs_mono() {
    // Créez un exemple de données d'entrée
    let input_data: Vec<i16> = vec![123, 456, 789, -321, 654, -987];
    let channels = 1;

    // Appelez la fonction pour obtenir le résultat
    let result = i16_vec_to_vecs(&input_data, channels);

    // Vérifiez que le résultat a le nombre attendu de canaux
    assert_eq!(result.len(), channels);

    // Vérifiez le contenu du résultat
    assert_eq!(result[0], vec![123.0, 456.0, 789.0, -321.0, 654.0, -987.0]);
  }

  /**
   * ? buffer_to_vecs Unit Tests
   */

  #[test]
  // fn test_buffer_to_vecs_single_channel() {
  //   let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  //   let mut input_buffer = std::io::Cursor::new(data);

  //   let channels = 1;
  //   let result = buffer_to_vecs(&mut input_buffer, channels);
  //   // mono so all inside same deep vec
  //   let expected_result: Vec<Vec<f64>> = vec![vec![
  //     i16::from_le_bytes([1, 2]).into(),
  //     i16::from_le_bytes([3, 4]).into(),
  //     i16::from_le_bytes([5, 6]).into(),
  //     i16::from_le_bytes([7, 8]).into(),
  //     i16::from_le_bytes([9, 10]).into(),
  //     i16::from_le_bytes([11, 12]).into(),
  //     i16::from_le_bytes([13, 14]).into(),
  //     i16::from_le_bytes([15, 16]).into(),
  //     // f64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
  //     // f64::from_le_bytes([9, 10, 11, 12, 13, 14, 15, 16]),
  //   ]];
  //   assert_eq!(result, expected_result);
  // }

  // #[test]
  // fn test_buffer_to_vecs_multiple_channels() {
  //   let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
  //   let mut input_buffer = std::io::Cursor::new(data);

  //   let channels = 2;
  //   let result = buffer_to_vecs(&mut input_buffer, channels);

  //   // stereo so vec of vec for channels
  //   let expected_result: Vec<Vec<f64>> = vec![
  //     vec![f64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8])],
  //     vec![f64::from_le_bytes([9, 10, 11, 12, 13, 14, 15, 16])],
  //   ];
  //   assert_eq!(result, expected_result);
  // }

  // #[test]
  // fn test_buffer_to_vecs_empty_input() {
  //   let data: &[u8] = &[];
  //   let mut input_buffer = std::io::Cursor::new(data);

  //   let channels = 1;
  //   let result = buffer_to_vecs(&mut input_buffer, channels);

  //   let expected_result: Vec<Vec<f64>> = vec![vec![]];
  //   assert_eq!(result, expected_result);
  // }

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

  /**
   * ? append_frames Unit Tests
   */

  #[test]
  fn test_append_frames() {
    let mut audio_buffers: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let additional_frames: Vec<Vec<f64>> = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
    let num_frames_to_append = 1;

    append_frames(&mut audio_buffers, &additional_frames, num_frames_to_append);

    assert_eq!(audio_buffers[0], vec![1.0, 2.0, 5.0]);
    assert_eq!(audio_buffers[1], vec![3.0, 4.0, 7.0]);
  }
  use std::fs;
  use tempfile::tempdir;
  #[test]
  fn test_write_frames_to_disk() {
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
