#[macro_use]
extern crate napi_derive;
extern crate rubato;
use log::debug;
use rubato::{implement_resampler, FastFixedIn, PolynomialDegree};

use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::{Read, Seek};
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::time::Instant;
use std::{env, vec};

use napi::Env;
use napi::{bindgen_prelude::*, JsObject};
use napi_derive::napi;

const BYTE_PER_SAMPLE: usize = 8;

implement_resampler!(SliceResampler, &[&[T]], &mut [Vec<T>]);

#[napi]
pub fn re_sample_buffers(
    input_buffer: Buffer,
    input_sample_rate: u16,
    output_sample_rate: u16,
    channels: u8,
) -> Buffer {
    // ? We may do  better via not copying data but need libc::memcpy, check if it's needed
    let input_slice: Vec<u8> = input_buffer.iter().map(|element| *element as u8).collect();
    let mut read_buffer = Box::new(Cursor::new(&input_slice));
    let data = buffer_to_vecs(&mut read_buffer, channels as usize);

    // let mut input_stereo: Vec<Vec<f32>> = vec![vec![], vec![]];

    // // Remplissez les vecteurs internes avec les données audio stéréo.
    // for (i, sample) in input_slice.iter().enumerate() {
    //     input_stereo[i % channels as usize].push(*sample);
    // }

    println!(
        " Size of input_slice {} and {}",
        input_slice.len(),
        data[1].len()
    );

    // // Getting the length of the input buffer
    // let _buffer_len = input_stereo[0].len();

    // // Calling your re_sample_audio_buffer function to get the output data
    let output_data = re_sample_audio_buffer(
        data,
        input_sample_rate,
        output_sample_rate,
        channels,
        channels,
    );

    output_data.into()
}

fn re_sample_audio_buffer(
    buffer: Vec<Vec<f64>>,
    input_sample_rate: u16,
    output_sample_rate: u16,
    input_channels: u8,
    output_channels: u8,
) -> Vec<u8> {
    println!("buffer size {}", buffer.len());

    let fs_in = input_sample_rate as usize;
    let channels = input_channels as usize;
    let nbr_input_frames = buffer[0].len(); // ? because for stereo
    let duration_total = Instant::now();

    let fs_out = output_sample_rate;
    debug!("Sample {} for output {}", &fs_in, &fs_out);

    // Create buffer for storing output
    let mut outdata = vec![
        Vec::with_capacity(
            2 * (nbr_input_frames as f64 * fs_out as f64 / fs_in as f64) as usize
        );
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

    skip_frames(outdata, resampler_delay, nbr_output_frames)
}

// F64 is required, panic if f32
fn buffer_to_vecs<R: Read>(input_buffer: &mut R, channels: usize) -> Vec<Vec<f64>> {
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
            let value = f64::from_le_bytes(buffer.as_slice().try_into().unwrap());
            wf.push(value);
        }
    }
    wfs
}

fn skip_frames(frames: Vec<Vec<f64>>, frames_to_skip: usize, frames_to_write: usize) -> Vec<u8> {
    let mut collected_data: Vec<u8> = Vec::new();
    let channels = frames.len();
    let end = frames_to_skip + frames_to_write;
    for frame_to_skip in frames_to_skip..end {
        for frame in frames.iter().take(channels) {
            let value64 = frame[frame_to_skip];
            let bytes = value64.to_le_bytes();
            collected_data.extend_from_slice(&bytes);
        }
    }
    collected_data
}

fn append_frames(buffers: &mut [Vec<f64>], additional: &[Vec<f64>], nbr_frames: usize) {
    buffers
        .iter_mut()
        .zip(additional.iter())
        .for_each(|(b, a)| b.extend_from_slice(&a[..nbr_frames]));
}
