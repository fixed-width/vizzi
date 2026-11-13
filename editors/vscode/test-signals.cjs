const fs=require('node:fs/promises');
async function writeSignal(file,value) {
  const temporary=file+'.tmp-'+process.pid;
  await fs.writeFile(temporary,JSON.stringify(value));
  await fs.rename(temporary,file);
}
module.exports={writeSignal};
