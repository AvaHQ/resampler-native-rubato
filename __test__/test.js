let i16Arr = new Int16Array([-32767, 42, 8, 12, 32767, 12243]);
let Buffi16 = Buffer.from(i16Arr.buffer);
// Buffi16.buffer //?
Buffi16.byteOffset //? 
