import {
  reSampleBuffers,
  reSampleAudioFile,
  reSampleInt16Buffer,
} from "../index.js";
import fs from "fs";
import { readFile } from "fs/promises";
import axios from "axios";
import { resolve } from "path";
import { exec as ExecOld } from "child_process";
import util from "util";

const exec = util.promisify(ExecOld);
const fileUrl =
  "https://upload.wikimedia.org/wikipedia/commons/f/fc/04_Faisle_Di_Ghadi_-_Paramjit_Maan.ogg";
const outputPathOGG = resolve(__dirname, "./output/test-audio-talk.ogg");
const outputPathRawInt16 = resolve(
  __dirname,
  "./output/test-audio-talk-int16.raw"
);
const outputPathRawf64 = resolve(__dirname, "./output/test-audio-talk-f64.raw");
const outputPathFile = resolve(__dirname, "./output/file-f64-output.raw");
const outputPathInt16 = resolve(__dirname, "./output/int16-output.raw");
const outputBuffer = resolve(__dirname, "./output/buffer-f64-output.raw");

let dataInt16: Buffer;
let dataF64: Buffer;

beforeAll(async () => {
  try {
    await downloadFile(fileUrl, outputPathOGG);
    console.log("Finished downloaded file ..");
    await runSoxCommand(outputPathOGG, outputPathRawInt16);
    await runSoxCommand(outputPathOGG, outputPathRawf64);
    console.log("Finished converting file to raw .. starting tests");
    dataInt16 = await readFile(outputPathRawInt16);
    dataF64 = await readFile(outputPathRawf64);
  } catch (error) {
    console.error(`error : ${error}`);
  }
}, 60000);

afterAll(async () => {
  await converToWavToCheck();
}, 60000);

describe("Native", () => {
  test("It Should be able to re-sampler INT16ARRAY in a correct time", () => {
    // TODO In fact fr the moment IMHO this is not a correct time, it took 4x time slower than buffer resampler
    let int16ArrayReSampleStartTime = Date.now();
    const resInt16 = fromIntInt16Buffer(dataInt16);
    let int16ArrayReSampleEndTime = Date.now();
    expect(
      int16ArrayReSampleEndTime - int16ArrayReSampleStartTime
    ).toBeLessThan(10000); // ? No regression test, should not be > 10s
    expect(resInt16.length).toEqual(270653648);
  }, 15000);
  test("It Should be able to re-sampler BUFFER in a correct time", () => {
    let bufferReSampleStartTime = Date.now();
    const resBuffer = fromBuffer(dataF64);
    let bufferReSampleEndTime = Date.now();
    expect(bufferReSampleEndTime - bufferReSampleStartTime).toBeLessThan(5000); // ? No regression test, should not be > 10s
    expect(resBuffer.length).toEqual(1082614592);
  }, 10000);
  test("It Should be able to re-sampler FILE in a correct time", () => {
    let fileReSampleStartTime = Date.now();
    fromFile(outputPathRawf64);
    let fileReSampleEndTime = Date.now();
    expect(fileReSampleEndTime - fileReSampleStartTime).toBeLessThan(2500); // ? No regression test, should not be > 10s
    expect(fs.existsSync(outputPathFile)).toBe(true);
  }, 10000);
});

async function downloadFile(url: string, outputPath: string) {
  if (fs.existsSync(outputPath)) {
    console.log(`File ${outputPath} alreayd exists.`);
    return;
  }

  try {
    const response = await axios.get(url, { responseType: "stream" });

    const writer = fs.createWriteStream(outputPath);

    response.data.pipe(writer);

    return new Promise((resolve, reject) => {
      writer.on("finish", resolve);
      writer.on("error", reject);
    });
  } catch (error) {
    console.error(error);
  }
}

function fromIntInt16Buffer(data: Buffer) {
  console.time("int16ArrayReSample");
  const resInt16 = reSampleInt16Buffer({
    inputInt16Array: data,
    argsAudioToReSample: {
      channels: 2,
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
    },
  });
  console.timeEnd("int16ArrayReSample");
  fs.writeFileSync(outputPathInt16, resInt16);
  return resInt16;
}

function fromBuffer(data: Buffer) {
  console.log("NODE- Buffer length", data.length);
  console.time("bufferReSample");
  const resBuffer = reSampleBuffers({
    inputBuffer: data,
    argsAudioToReSample: {
      channels: 2,
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
    },
  });
  console.timeEnd("bufferReSample");
  fs.writeFileSync(outputBuffer, resBuffer);
  return resBuffer;
}

function fromFile(inputRawPath: string) {
  console.time("fileResample");
  reSampleAudioFile({
    outputPath: outputPathFile,
    inputRawPath,
    argsAudioToReSample: {
      channels: 2,
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
    },
  });
  console.timeEnd("fileResample");
}

async function runSoxCommand(inputFilePath: string, outputFilePath: string) {
  let type = outputFilePath.includes("f64")
    ? "floating-point"
    : "signed-integer";
  let size = outputFilePath.includes("f64") ? "64" : "16";
  const command = `sox ${inputFilePath} -e ${type} -b ${size} ${outputFilePath}`;

  console.log(command);
  const { stderr } = await exec(command);

  if (stderr) {
    console.error(`SOX error  : ${stderr}`);
    return;
  }

  console.log("Sox conversion to raw file done");
}

async function converToWavToCheck() {
  const files = [outputBuffer, outputPathFile, outputPathInt16];
  const proms = files.map((file) => {
    let type = file.includes("f64") ? "floating-point" : "signed-integer";
    let size = file.includes("f64") ? "64" : "16";
    const command = `sox -e ${type} -b ${size} -r 16000 -c 2 ${file} -e signed-integer -b 16 ${file.replace(
      ".raw",
      ".wav"
    )}`;
    console.log(command);
    return exec(command);
  });
  return Promise.allSettled(proms);
}
