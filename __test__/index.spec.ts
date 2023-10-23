import {
  reSampleBuffers,
  reSampleAudioFile,
  reSampleInt16Array,
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
const outputPathRaw = resolve(__dirname, "./output/test-audio-talk.raw");
const outputPathFile = resolve(__dirname, "./output/file-output.raw");
const outputPathInt16 = resolve(__dirname, "./output/int16-output.raw");
const outputBuffer = resolve(__dirname, "./output/buffer-output.raw");

downloadFile(fileUrl, outputPathOGG)
  .then(async () => {
    console.log("Finished downloaded file ..");
    await runSoxCommand(outputPathOGG, outputPathRaw);
    console.log("Finished converting file to raw .. starting tests");
    let data = await readFile(outputPathRaw);
    fromIntArray(data);
    fromBuffer(data);
    fromFile(outputPathRaw);
    await converToWavToCheck();
  })
  .catch((error) => {
    console.error(`Erreur : ${error}`);
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

function fromIntArray(data: Buffer) {
  let dataInt16Array = new Int16Array(data.buffer);
  console.log("NODE- dataInt16Array length", dataInt16Array.length);

  console.time("int16ArrayReSample");
  const resInt16 = reSampleInt16Array({
    inputInt16Array: dataInt16Array,
    argsAudioToReSample: {
      channels: 2,
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
    },
  });
  console.timeEnd("int16ArrayReSample");
  fs.writeFileSync(outputPathInt16, resInt16);
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
  const command = `sox ${inputFilePath} -e floating-point -b 64 ${outputFilePath}`;

  const { stderr, stdout } = await exec(command);

  if (stderr) {
    console.error(`SOX error  : ${stderr}`);
    return;
  }

  console.log("Sox conversion to raw file done");
}

async function converToWavToCheck() {
  const files = [outputBuffer, outputPathFile, outputPathInt16];
  const proms = files.map((file) => {
    const command = `sox -e floating-point -b 64 -r 16000 -c 2 ${file}  -e signed-integer -b 16 ${file.replace(
      ".raw",
      ".wav"
    )}`;
    return exec(command);
  });
  return Promise.allSettled(proms);
}
