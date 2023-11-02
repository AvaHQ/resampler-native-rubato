import { reSampleBuffers, reSampleInt16Buffer, DataType } from "../index.js";
import fs, { unlinkSync } from "fs";
import { readFile, writeFile } from "fs/promises";
import axios from "axios";
import { resolve } from "path";
import { exec as ExecOld } from "child_process";
import util from "util";

const exec = util.promisify(ExecOld);
const OUT_DIR = resolve(__dirname, `./output`);
const OUT_DIR_FILE = (filename: string) => resolve(`${OUT_DIR}/${filename}`);

// all the data of those type came from the file function
type infoForResample = {
  id: number;
  comments?: string;
  channels: "mono" | "stereo";
  format: "ogg" | "wav";
  sampleRateInput: 44000 | 44100 | 48000;
  sampleRateOutput: 16000 | 24000 | 32000 | 44000 | 44100 | 48000;
  expectMaxTimeToConvert: number; //in ms
  expectedSize: number; //in nb of frames
};
type FilesToResamples = {
  [key: `https://${string}`]: infoForResample;
};

const files_to_resamples: FilesToResamples = {
  "https://upload.wikimedia.org/wikipedia/commons/7/7e/Fiche_technique_Ficus_Benjamina.ogg":
    {
      id: 1,
      channels: "stereo",
      format: "ogg",
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
      comments:
        "This file could have error in frames conversion because of its structur",
      expectMaxTimeToConvert: 60,
      expectedSize: 5890808,
    },
  "https://upload.wikimedia.org/wikipedia/commons/d/de/Fr-à_bientôt_%21.ogg": {
    id: 2,
    channels: "mono",
    format: "ogg",
    sampleRateInput: 44000,
    sampleRateOutput: 32000,
    comments:
      "It's a short mono (<1s) so make sure the output don't hav acceleration of the voice",
    expectMaxTimeToConvert: 20,
    expectedSize: 166400,
  },
  "https://upload.wikimedia.org/wikipedia/commons/f/f7/%22Le_village_de_Mollon_dans_l%27Ain%22_prononcé_par_un_habitant_%28dans_la_rue%29.ogg":
    {
      id: 3,
      format: "ogg",
      sampleRateInput: 44100,
      sampleRateOutput: 24000,
      channels: "mono",
      expectMaxTimeToConvert: 300,
      expectedSize: 240604,
      comments:
        "It's a short mono (~1s) so make sure the output don't hav acceleration of the voice",
    },
  "https://upload.wikimedia.org/wikipedia/commons/e/ec/Eric_Walter_-_voix.ogg":
    {
      id: 4,
      format: "ogg",
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
      channels: "stereo",
      expectMaxTimeToConvert: 500,
      expectedSize: 1660872,
    },
  "https://upload.wikimedia.org/wikipedia/commons/f/fc/04_Faisle_Di_Ghadi_-_Paramjit_Maan.ogg":
    {
      id: 5,
      format: "wav",
      sampleRateInput: 44100,
      sampleRateOutput: 16000,
      channels: "stereo",
      comments: "Its a big file of 50mb ogg for 35mn audio",
      expectMaxTimeToConvert: 6500,
      expectedSize: 270653648,
    },
  "https://upload.wikimedia.org/wikipedia/commons/f/f4/18-dic.-23.12.wav": {
    id: 6,
    format: "wav",
    sampleRateInput: 48000,
    sampleRateOutput: 16000,
    channels: "stereo",
    comments: "This wav use little endian",
    expectMaxTimeToConvert: 500,
    expectedSize: 7034192,
  },
  // TODO: In the future should work with BE as like LE , so add a test
};

beforeAll(async () => {
  try {
    // for each url download it name as getBaseName() => `sample-{channels}-{sampleRateInput}.${format}`
    // for int16 f32 create the getRawBaseName() => `sample-${channels}-${sampleRateInput}-${format}-${numberType}.raw`
    await cleanOutputFolder("start");
    const files_to_dl = Object.entries(files_to_resamples).map(
      ([url, data]) => {
        const output_base = OUT_DIR_FILE(getBaseName(data));
        return () => downloadFile(url, output_base);
      }
    );
    await Promise.all(files_to_dl.map((f) => f()));

    console.log("ALL files downloaded");

    for (const [_, data] of Object.entries(files_to_resamples)) {
      const output_base = OUT_DIR_FILE(getBaseName(data));

      const ouputRawBaseI16 = OUT_DIR_FILE(getRawBaseName(data, DataType.I16));
      const ouputRawBaseF32 = OUT_DIR_FILE(getRawBaseName(data, DataType.F32));
      await runSoxCommandOnBase(output_base, ouputRawBaseI16, DataType.I16);
      await runSoxCommandOnBase(output_base, ouputRawBaseF32, DataType.F32);
    }

    console.log("Finished converted all samples to raw");
  } catch (error) {
    console.error(`error : ${error}`);
  }
}, 120000); // long timeout because could need to download a 50mb file

afterAll(async () => {
  await cleanOutputFolder("end");
});

/**
 * ? Those tests work with a 50.1MB OGG file, 30mn of audio talk of a woman, corresponding of ~1GB of raw f32 data or ~370MB i16
 * ? from 44100Hz to 16Khz stereo
 */
describe("NAPI -  Rubato Module", () => {
  Object.entries(files_to_resamples).forEach(([_, data]) => {
    const {
      channels: channelsStr,
      format,
      sampleRateInput,
      sampleRateOutput,
      expectMaxTimeToConvert,
      expectedSize,
      id,
    } = data;
    test(`${format.toUpperCase()} ${channelsStr} ${sampleRateInput} -> ${sampleRateOutput}`, async () => {
      let channels = channelsStr === "mono" ? 1 : 2;
      const input_raw_base_i16 = OUT_DIR_FILE(
        getRawBaseName(data, DataType.I16)
      );
      const input_raw_base_f32 = OUT_DIR_FILE(
        getRawBaseName(data, DataType.F32)
      );
      const [bufferI16, bufferF32] = await Promise.all([
        readFile(input_raw_base_i16),
        readFile(input_raw_base_f32),
      ]);
      let startF32 = Date.now();
      const convertedBuffF32 = reSampleBuffers({
        inputBuffer: bufferF32,
        argsAudioToReSample: {
          channels,
          sampleRateInput,
          sampleRateOutput,
        },
      });
      let endF32 = Date.now();
      if (!process.env.GITHUB_ACTIONS) {
        expect(endF32 - startF32).toBeLessThan(expectMaxTimeToConvert); // time on CI depend on running usage .. not doing this
      }
      console.log(`SIZE for ${id} - f32:`, convertedBuffF32.length);
      let startI16 = Date.now();
      const convertedBuffI16 = reSampleInt16Buffer({
        inputInt16Buffer: bufferI16,
        argsAudioToReSample: {
          channels,
          sampleRateInput,
          sampleRateOutput,
        },
      });
      console.log(`SIZE for ${id} - i16:`, convertedBuffI16.length);
      let endI16 = Date.now();
      expect(endI16 - startI16).toBeLessThan(expectMaxTimeToConvert);
      // Some frames can be lost but should be in range with ~10% max
      expect(convertedBuffI16.length).toBeLessThan(
        expectedSize / 2 + expectedSize * 0.1
      );
      expect(convertedBuffI16.length).toBeGreaterThan(
        expectedSize / 2 - expectedSize * 0.1
      );

      if (!process.env.GITHUB_ACTIONS) {
        const outputRawConvertedI16 = OUT_DIR_FILE(
          getConvertedRawName(data, DataType.I16)
        );
        const outputRawConvertedF32 = OUT_DIR_FILE(
          getConvertedRawName(data, DataType.F32)
        );
        const outputFinalConvertedI16 = OUT_DIR_FILE(
          getConvertedName(data, DataType.I16)
        );
        const outputFinalConvertedF32 = OUT_DIR_FILE(
          getConvertedName(data, DataType.F32)
        );
        await Promise.all([
          writeFile(outputRawConvertedI16, convertedBuffI16),
          writeFile(outputRawConvertedF32, convertedBuffF32),
        ]);
        await runSoxCommandOnConverted(
          outputRawConvertedI16,
          outputFinalConvertedI16,
          DataType.I16,
          sampleRateOutput,
          channels
        );
        await runSoxCommandOnConverted(
          outputRawConvertedF32,
          outputFinalConvertedF32,
          DataType.F32,
          sampleRateOutput,
          channels
        );
      }
    }, 50000); // depending on os the ttest on the big file could be slow
  });
});

/**
 * Will download the entry fiel for test, will not re-dl it if already present
 * @param url link to .wav/ogg file to download
 * @param outputPath Path to save the file
 * @returns  void
 */
async function downloadFile(url: string, outputPath: string) {
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
async function runSoxCommandOnBase(
  inputFilePath: string,
  outputFilePath: string,
  dataType: DataType
) {
  let type = dataType === DataType.F32 ? "floating-point" : "signed-integer";
  let size = dataType === DataType.F32 ? "32" : "16";
  const command = `sox ${inputFilePath} -e ${type} -b ${size} ${outputFilePath}`;
  const { stderr } = await exec(command);

  if (stderr) {
    console.log(`SOX error  : ${stderr}`);
  }
}

async function runSoxCommandOnConverted(
  inputFilePath: string,
  outputFilePath: string,
  dataType: DataType,
  sampleRateOutput: number,
  channels: number
) {
  let type = dataType === DataType.F32 ? "floating-point" : "signed-integer";
  let size = dataType === DataType.F32 ? "32" : "16";
  const command = `sox  -e ${type} -b ${size} -r ${sampleRateOutput} -c ${channels} ${inputFilePath} -e signed-integer -b 16 ${outputFilePath}`;
  const { stderr } = await exec(command);

  if (stderr) {
    console.log(`SOX error  : ${stderr}`);
  }
}

function getBaseName({
  id,
  channels,
  format,
  sampleRateInput,
}: infoForResample) {
  return `sample-${id}-${channels}-${sampleRateInput}.${format}`;
}
function getRawBaseName(
  { id, channels, format, sampleRateInput }: infoForResample,
  dataType: DataType
) {
  return `sample-${id}-${channels}-${sampleRateInput}-${format}-${dataType}.raw`;
}
function getConvertedRawName(
  { id, channels, format, sampleRateInput, sampleRateOutput }: infoForResample,
  dataType: DataType
) {
  return `converted-${id}-${channels}-${sampleRateInput}-${sampleRateOutput}-${format}-${dataType}.raw`;
}
function getConvertedName(
  { id, channels, sampleRateInput, sampleRateOutput }: infoForResample,
  dataType: DataType
) {
  let dataTypeStr = dataType === DataType.I16 ? "i16" : "f32";
  return `converted-${id}-${channels}-${sampleRateInput}-${sampleRateOutput}-${dataTypeStr}.wav`; // wave for all because ogg does not support some sampleRate !
}

// When tests start we keep only the downloaded file, when tests finished  we remove all raw and keep final .wav to be able to listen to them
async function cleanOutputFolder(type: "start" | "end") {
  fs.readdirSync(OUT_DIR).forEach((filename) => {
    let shouldKeepThisFile =
      type === "end"
        ? !filename.includes(".raw") &&
          filename.includes("converted") &&
          !filename.includes(".gitkeep")
        : false;
    if (shouldKeepThisFile) {
      return;
    }
    let filePath = OUT_DIR_FILE(filename);
    unlinkSync(filePath);
  });
}
