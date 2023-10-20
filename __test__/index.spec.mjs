
import { reSampleBuffers, reSampleAudioFile } from '../index.js'
import fs from "fs"


let inputRawPath = "/Users/dieudonn/Downloads/big-talk.raw";
let outputPath = "/Users/dieudonn/Downloads/big-talk-resampled-2.raw";


// Buffer way

fs.readFile(inputRawPath, (err, data) => {
  if (err) {
    console.error(err);
    return;
  }

  let test = new Int16Array(data.buffer);
  let test2 = Buffer.from(test);

  console.log('File loaded');
  console.time("bufferReSample");
  const res = reSampleBuffers({inputBuffer: data, argsAudioToReSample: {channels: 2, sampleRateInput: 44100, sampleRateOutput: 32000}});
  console.timeEnd("bufferReSample");
  
  fs.writeFileSync("/Users/dieudonn/Downloads/big-talk-resampled-1.raw", res)
  // File way for testing
  console.time("fileResample");
  reSampleAudioFile({outputPath, inputRawPath, argsAudioToReSample: {channels: 2, sampleRateInput: 44100, sampleRateOutput: 48000}})
  console.timeEnd("fileResample");
});


