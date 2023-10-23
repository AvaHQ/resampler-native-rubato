
import { reSampleBuffers, reSampleAudioFile,reSampleInt16Array } from '../index.js'
import fs from "fs"


let inputRawPath = "/Users/dieudonn/Downloads/big-talk.raw";
let outputPath = "/Users/dieudonn/Downloads/big-talk-resampled-2.raw";


// Buffer way

fs.readFile(inputRawPath, (err, data) => {
  if (err) {
    console.error(err);
    return;
  }
  console.log('File loaded');
  
  let dataInt16Array = new Int16Array(data.buffer);
  console.log('Array16 base length is ', dataInt16Array.length);
  
  console.time("int16ArrayReSample");
  const resInt16 = reSampleInt16Array({inputInt16Array: dataInt16Array, argsAudioToReSample:{channels: 2, sampleRateInput: 44100, sampleRateOutput: 16000}});
  console.timeEnd("int16ArrayReSample");
  console.log('resInt16 base length is ', resInt16.length);
  fs.writeFileSync("/Users/dieudonn/Downloads/big-talk-resampled-1.raw", resInt16)


  // console.time("bufferReSample");
  // const resBuffer = reSampleBuffers({inputBuffer: data, argsAudioToReSample: {channels: 2, sampleRateInput: 44100, sampleRateOutput: 32000}});
  // console.timeEnd("bufferReSample");
  
  // fs.writeFileSync("/Users/dieudonn/Downloads/big-talk-resampled-1.raw", resBuffer)
  // File way for testing
  // console.time("fileResample");
  // reSampleAudioFile({outputPath, inputRawPath, argsAudioToReSample: {channels: 2, sampleRateInput: 44100, sampleRateOutput: 16000}})
  // console.timeEnd("fileResample");
});


