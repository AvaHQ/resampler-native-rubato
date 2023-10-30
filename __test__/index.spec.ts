import {
  reSampleBuffers,
  reSampleAudioFile,
  reSampleInt16Buffer,
  DataType,
} from "../index.js";
import fs, { unlinkSync, writeFileSync } from "fs";
import { readFile, writeFile } from "fs/promises";
import axios from "axios";
import { resolve } from "path";
import { exec as ExecOld } from "child_process";
import util from "util";

const exec = util.promisify(ExecOld);
const OUT_DIR = resolve(__dirname, `./output`);
const OUT_DIR_FILE = (filename: string) => resolve(`${OUT_DIR}/${filename}`);
// const WAV_OR_OGG_URL =
// "https://upload.wikimedia.org/wikipedia/commons/7/7e/Fiche_technique_Ficus_Benjamina.ogg"; // error End frames_to_skip + frames_to_write 2029572 is above the length of frames which are 2"
const WAV_OR_OGG_URL =
  "https://upload.wikimedia.org/wikipedia/commons/f/f7/%22Le_village_de_Mollon_dans_l%27Ain%22_prononcé_par_un_habitant_%28dans_la_rue%29.ogg"; // Error: the spitch change
// const WAV_OR_OGG_URL =
//   "https://upload.wikimedia.org/wikipedia/commons/f/fc/04_Faisle_Di_Ghadi_-_Paramjit_Maan.ogg";
// const WAV_OR_OGG_URL =
//   "https://archive.org/download/Rpp-Episode16WavVersion/rpp16.wav";
const BASE_AUDIO_NAME = "sample-talk.ogg";
const BASE_AUDIO_FILE = OUT_DIR_FILE(BASE_AUDIO_NAME);
const BASE_RAW_I16 = OUT_DIR_FILE("sample-talk-int16.raw");
const BASE_RAW_F32 = OUT_DIR_FILE("sample-talk-f32.raw");

beforeAll(async () => {
  try {
    await cleanOutputFolder("start");
    await downloadFile(WAV_OR_OGG_URL, BASE_AUDIO_FILE);
    console.log("Finished downloaded file ..");
    await runSoxCommand(BASE_AUDIO_FILE, BASE_RAW_I16);
    await runSoxCommand(BASE_AUDIO_FILE, BASE_RAW_F32);
    console.log("Finished converting file to raw .. starting tests");
  } catch (error) {
    console.error(`error : ${error}`);
  }
}, 60000); // long timeout because could need to download a 50mb file

afterAll(async () => {
  await cleanOutputFolder("end");
});

/**
 * ? Those tests work with a 50.1MB OGG file, 30mn of audio talk of a woman, corresponding of ~1GB of raw f32 data or ~370MB i16
 * ? from 44100Hz to 16Khz stereo
 */
describe("NAPI -  Rubato Module", () => {
  test("Should resample Buffer of int16 data in an acceptable time", async () => {
    let int16BufferReSampleStart = Date.now();
    const dataInt16 = await readFile(BASE_RAW_I16);
    console.time("int16ArrayReSample");
    const resampleBufferInt16 = reSampleInt16Buffer({
      inputInt16Buffer: dataInt16,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 44100,
      },
    });
    console.timeEnd("int16ArrayReSample");
    let int16BufferReSampleEnd = Date.now();
    // ? No regression test, should not be > 10s
    expect(int16BufferReSampleEnd - int16BufferReSampleStart).toBeLessThan(
      10000
    );
    const reSampleBufferInt16Path = OUT_DIR_FILE("buffer-int16.raw");
    writeFileSync(reSampleBufferInt16Path, resampleBufferInt16);
    const reSampleBufferInt16PathWav = reSampleBufferInt16Path.replace(
      ".raw",
      ".wav"
    );
    await exec(
      `sox  -e signed-integer -b 16 -r 44100 -c 2 ${reSampleBufferInt16Path} -e signed-integer -b 16 ${reSampleBufferInt16PathWav}`
    );
  }, 15000);

  test("Should re-sample Buffer (f32) in an acceptable time", async () => {
    let bufferReSampleStart = Date.now();
    const dataF32 = await readFile(BASE_RAW_F32);
    console.time("bufferReSample");
    const resampleBufferF32 = reSampleBuffers({
      inputBuffer: dataF32,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("bufferReSample");
    const resampleBufferF32Path = OUT_DIR_FILE("buffer-f32.raw");
    await writeFile(resampleBufferF32Path, resampleBufferF32);
    let bufferReSampleEndTime = Date.now();
    // ? No regression test, should not be > 5s
    expect(bufferReSampleEndTime - bufferReSampleStart).toBeLessThan(5000);
    const reSampleBufferF32PathWav = resampleBufferF32Path.replace(
      ".raw",
      ".wav"
    );
    await exec(
      `sox  -e floating-point -b 32 -r 16000 -c 2 ${resampleBufferF32Path} -e signed-integer -b 16 ${reSampleBufferF32PathWav}`
    );
    // expect(reSampledBuff.length).toEqual(1082614592);
  }, 15000);

  test("Should re-sample via File path (f32) in an acceptable time", async () => {
    let fileReSampleStartTime = Date.now();
    console.time("fileResample");
    const resampleF32PathFile = OUT_DIR_FILE("file-f32.raw");
    expect(fs.existsSync(resampleF32PathFile)).toBe(false);
    reSampleAudioFile({
      outputPath: resampleF32PathFile,
      typeOfBinData: DataType.F32,
      inputRawPath: BASE_RAW_F32,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("fileResample");
    let fileReSampleEndTime = Date.now();
    expect(fileReSampleEndTime - fileReSampleStartTime).toBeLessThan(2500); // ? No regression test, should not be > 2.5s
    expect(fs.existsSync(resampleF32PathFile)).toBe(true);
    const reSampleFileF32PathWav = resampleF32PathFile.replace(".raw", ".wav");
    await exec(
      `sox -e floating-point -b 32 -r 16000 -c 2 ${resampleF32PathFile} -e signed-integer -b 16 ${reSampleFileF32PathWav}`
    );
  }, 10000);

  test("Should re-sample via File path (i16) in an acceptable time", async () => {
    let fileReSampleStartTime = Date.now();
    console.time("fileResample");
    const resampleI16PathFile = OUT_DIR_FILE("file-i16.raw");
    expect(fs.existsSync(resampleI16PathFile)).toBe(false);
    reSampleAudioFile({
      outputPath: resampleI16PathFile,
      typeOfBinData: DataType.I16,
      inputRawPath: BASE_RAW_I16,
      argsAudioToReSample: {
        channels: 2,
        sampleRateInput: 44100,
        sampleRateOutput: 16000,
      },
    });
    console.timeEnd("fileResample");
    let fileReSampleEndTime = Date.now();
    expect(fileReSampleEndTime - fileReSampleStartTime).toBeLessThan(2500); // ? No regression test, should not be > 2.5s
    expect(fs.existsSync(resampleI16PathFile)).toBe(true);
    const resampleI16PathFileWav = resampleI16PathFile.replace(".raw", ".wav");
    await exec(
      `sox -e signed-integer -b 16 -r 16000 -c 2 ${resampleI16PathFile} -e signed-integer -b 16 ${resampleI16PathFileWav}`
    );
  }, 10000);
});

/**
 * Will download the entry fiel for test, will not re-dl it if already present
 * @param url link to .wav/ogg file to download
 * @param outputPath Path to save the file
 * @returns  void
 */
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

/**
 * Will convert from ogg/wav.. to raw audio
 * depending on data_type (i16/f32) if will generate a file with different name
 * @param inputFilePath .OGG/.WAV entry file
 * @param outputFilePath .RAW converted entry file
 * @returns void
 */
async function runSoxCommand(inputFilePath: string, outputFilePath: string) {
  let type = outputFilePath.includes("f32")
    ? "floating-point"
    : "signed-integer";
  let size = outputFilePath.includes("f32") ? "32" : "16";
  const command = `sox ${inputFilePath} -e ${type} -b ${size} ${outputFilePath}`;
  const { stderr } = await exec(command);

  if (stderr) {
    console.error(`SOX error  : ${stderr}`);
    return;
  }

  console.log(`Sox conversion to raw file in ${size} done`);
}

// When tests start we keep only the downloaded file, when tests finished  we remove all raw and keep final .wav to be able to listen to them
async function cleanOutputFolder(type: "start" | "end") {
  fs.readdirSync(OUT_DIR).forEach((filename) => {
    let shouldKeepThisFile =
      type === "end"
        ? filename.includes(".wav") || filename === BASE_AUDIO_NAME
        : filename === BASE_AUDIO_NAME;
    console.log("BASE_AUDIO_NAME", BASE_AUDIO_NAME, shouldKeepThisFile);
    if (shouldKeepThisFile) {
      return;
    }
    let filePath = OUT_DIR_FILE(filename);
    unlinkSync(filePath);
  });
}
