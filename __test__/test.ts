import { readFileSync, writeFileSync } from "fs";
import { reSampleInt16Buffer } from "../";

const data = readFileSync("/Users/dieudonn/Dev/talk.raw");

const res = reSampleInt16Buffer({
  argsAudioToReSample: {
    channels: 2,
    sampleRateInput: 44100,
    sampleRateOutput: 16000,
  },
  inputInt16Array: data,
});

console.log("res", res.length);

writeFileSync("/Users/dieudonn/Dev/talk-re.raw", res);
