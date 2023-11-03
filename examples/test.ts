import { reSampleInt16Buffer } from "../";

const INPUT_SAMPLE_RATE = 44100;
const OUTPUT_SAMPLE_RATE = 16000;
const int16Array = new Int16Array([
  42, 123, -456, 789, 42, 123, -456, 789, 42, 123, -456, 789, 42, 123, -456,
  123,
]);
const buffer = Buffer.from(int16Array.buffer);

const res = reSampleInt16Buffer({
  argsAudioToReSample: {
    channels: 2,
    sampleRateInput: INPUT_SAMPLE_RATE,
    sampleRateOutput: OUTPUT_SAMPLE_RATE,
  },
  inputInt16Buffer: buffer,
});

console.log(
  `input sampleRate is ${INPUT_SAMPLE_RATE} and length of buffer is ${buffer.length} output sampleRate is ${OUTPUT_SAMPLE_RATE} and ${res.length} `
);
