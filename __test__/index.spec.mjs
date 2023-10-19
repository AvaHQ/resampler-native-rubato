
import { reSampleBuffers } from '../index.js'
import fs from "fs"


let file_in = "/Users/dieudonn/Downloads/large-sample-usa.raw";


fs.readFile(file_in, (err, data) => {
  if (err) {
    console.error('Erreur lors de la lecture du fichier audio :', err);
    return;
  }


  console.log('Fichier audio chargé avec succès !');
  console.log(typeof data, data.length);
  // const arr = new Int16Array(data.buffer,z);

  const res = reSampleBuffers(data, 44100, 16000, 2, 2);
  console.log(res);
  fs.writeFileSync("/Users/dieudonn/Downloads/large-sample-usa-napi.raw", res)

});
