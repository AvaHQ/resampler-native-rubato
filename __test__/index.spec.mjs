
import { reSampleBuffers, reSampleAudioFile } from '../index.js'
import fs from "fs"


let inputRawPath = "/Users/dieudonn/Downloads/big-talk.raw";
let outputPath = "/Users/dieudonn/Downloads/big-talk-resampled.raw";


// Buffer way

fs.readFile(file_in, (err, data) => {
  if (err) {
    console.error(err);
    return;
  }

  console.log('File loaded');
  console.time("bufferReSample");
  const res = reSampleBuffers({});
  console.timeEnd("bufferReSample");
  
  fs.writeFileSync("/Users/dieudonn/Downloads/large-resampled.raw", res)
  // File way for testing
  console.time("fileResample");
  reSampleAudioFile({outputPath, inputRawPath, argsAudioToReSample: {channels: 2, sampleRateInput: 44100, sampleRateOutput: 16000}})
  console.timeEnd("fileResample");
});


