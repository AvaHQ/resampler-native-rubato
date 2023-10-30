import {
  reSampleBuffers,
  reSampleAudioFile,
  reSampleInt16Buffer,
} from "../index.js";
import fs, { writeFileSync } from "fs";
import { readFile, writeFile } from "fs/promises";
import axios from "axios";
import { resolve } from "path";
import { exec as ExecOld } from "child_process";
import util from "util";

const exec = util.promisify(ExecOld);
const OUT_DIR = resolve(__dirname, `./output`);
const OUT_DIR_FILE = (filename: string) => resolve(`${OUT_DIR}/${filename}`);
const OGG_URL =
  "https://upload.wikimedia.org/wikipedia/commons/f/fc/04_Faisle_Di_Ghadi_-_Paramjit_Maan.ogg";
const OUTPUT_OGG = OUT_DIR_FILE("sample-talk.ogg");
const BASE_RAW_I16 = OUT_DIR_FILE("sample-talk-int16.raw");
const BASE_RAW_F64 = OUT_DIR_FILE("sample-talk-f64.raw");

beforeAll(async () => {
  try {
    await downloadFile(OGG_URL, OUTPUT_OGG);
    console.log("Finished downloaded file ..");
    // await runSoxCommand(OUTPUT_OGG, BASE_RAW_I16);
    // await runSoxCommand(OUTPUT_OGG, BASE_RAW_F64);
    console.log("Finished converting file to raw .. starting tests");
  } catch (error) {
    console.error(`error : ${error}`);
  }
}, 60000);

afterAll(async () => {
  // await converToWavToCheck();
}, 60000);

describe("Native", () => {
  test.only("Should resample Buffer of int16 data in an acceptable time", async () => {
    let int16BufferReSampleStart = Date.now();
    console.log(OUTPUT_OGG, BASE_RAW_I16);
    await exec(`sox ${OUTPUT_OGG} -e signed-integer -b 16 ${BASE_RAW_I16}`);
    const dataInt16 = await readFile(BASE_RAW_I16);
    console.time("int16ArrayReSample");
    const resInt16 = reSampleInt16Buffer({
      inputInt16Array: dataInt16,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("int16ArrayReSample");
    let int16BufferReSampleEnd = Date.now();
    // ? No regression test, should not be > 10s
    expect(int16BufferReSampleEnd - int16BufferReSampleStart).toBeLessThan(
      10000
    );
    const outputPathInt16 = OUT_DIR_FILE("buffer-int16.raw");
    writeFileSync(outputPathInt16, resInt16);
    const outputPathInt16Wav = outputPathInt16.replace(".raw", ".wav");
    await exec(
      `sox  -e signed-integer -b 16 -r 16000 -c 2 ${outputPathInt16} -e signed-integer -b 16 ${outputPathInt16Wav}`
    );
  }, 15000);

  test("Should re-sample Buffer (f64) in an acceptable time", async () => {
    let bufferReSampleStart = Date.now();
    const dataF64 = await readFile(BASE_RAW_I16);
    console.time("bufferReSample");
    const resamplerBufferF64 = reSampleBuffers({
      inputBuffer: dataF64,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("bufferReSample");
    const resampleBufferF64 = OUT_DIR_FILE("buffer-f64.raw");
    console.log("TEST 1 ", resampleBufferF64);
    await writeFile(resampleBufferF64, resamplerBufferF64);
    let bufferReSampleEndT = Date.now();
    // ? No regression test, should not be > 10s
    expect(bufferReSampleEndT - bufferReSampleStart).toBeLessThan(5000);
    // expect(reSampledBuff.length).toEqual(1082614592);
  }, 10000);

  test("Should re-sample via File path (f64) in an acceptable time", async () => {
    let fileReSampleStartTime = Date.now();
    console.time("fileResample");
    const resamplePathFile = OUT_DIR_FILE("file-f64.raw");
    console.log("TEST 2 ", resamplePathFile);
    expect(fs.existsSync(resamplePathFile)).toBe(false);
    reSampleAudioFile({
      outputPath: resamplePathFile,
      inputRawPath: BASE_RAW_F64,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("fileResample");
    let fileReSampleEndTime = Date.now();
    expect(fileReSampleEndTime - fileReSampleStartTime).toBeLessThan(2500); // ? No regression test, should not be > 10s
    expect(fs.existsSync(resamplePathFile)).toBe(true);
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

function fromFile(inputRawPath: string) {}

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
  console.log("converToWavToCheck");
  const rawsToWavs = fs.readdirSync(OUT_DIR).map((filename) => {
    const file = OUT_DIR_FILE(filename);
    // console.log("filename end ", file);
    if (file.includes("sample-") || !file.includes(".raw")) {
      console.log("not TAKE ", file);
      return;
    }
    console.log("TAKE ", file);
    let type = file.includes("f64") ? "floating-point" : "signed-integer";
    let size = file.includes("f64") ? "64" : "16";
    const command = `sox -e ${type} -b ${size} -r 16000 -c 2 ${file} -e signed-integer -b 16 ${file.replace(
      ".raw",
      ".wav"
    )}`;
    console.log(command);
    return exec(command);
  });
  return Promise.allSettled(rawsToWavs);
}
