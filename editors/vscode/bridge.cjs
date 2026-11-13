const net = require('node:net');
const crypto = require('node:crypto');
const fs = require('node:fs/promises');
const path = require('node:path');

const FRAME = 16 * 1024 * 1024;

async function startBridge({root, connectionFile, query}) {
  const token = crypto.randomBytes(32).toString('hex');
  const session = crypto.randomBytes(16).toString('hex');
  const sockets = new Set();
  let busy = false;
  let stopped = false;
  const server = net.createServer(socket => {
    if (sockets.size >= 4) { socket.destroy(); return; }
    sockets.add(socket);
    const timer = setTimeout(() => socket.destroy(), 12000);
    socket.once('close', () => {clearTimeout(timer); sockets.delete(socket);});
    socket.on('error', () => {});
    let chunks = [];
    let length = 0;
    let received = false;
    function send(result) {
      let frame = JSON.stringify({version:1,...result}) + '\n';
      if (Buffer.byteLength(frame) > FRAME) frame = JSON.stringify({version:1,status:'error',message:'Editor response exceeds allowance'}) + '\n';
      if (!socket.destroyed) socket.end(frame);
    }
    socket.on('data', async data => {
      if (received) {socket.destroy(); return;}
      length += data.length;
      if (length > FRAME) {socket.destroy(); return;}
      chunks.push(data);
      if (!data.includes(10)) return;
      received = true;
      const frame = Buffer.concat(chunks);
      chunks = [];
      let ownsQuery = false;
      try {
        if (frame.indexOf(10) !== frame.length - 1) throw new Error('Expected one request frame');
        const request = JSON.parse(frame.subarray(0, -1).toString('utf8'));
        if (request.version !== 1 || request.root !== root || typeof request.token !== 'string'
            || request.token.length !== 64 || !crypto.timingSafeEqual(Buffer.from(request.token), Buffer.from(token))) {
          throw new Error('Invalid editor connection credentials, root or protocol version');
        }
        if (!['definition','references','dependency-definition','dependency-references'].includes(request.method)) throw new Error('Unsupported editor operation');
        if (busy || stopped) throw new Error('Editor analysis is busy; retry navigation');
        busy = true;
        ownsQuery = true;
        const result = await query(request);
        if (stopped) throw new Error('Editor sharing stopped');
        send({...result,status:'ok',session});
      } catch (error) {
        send({status:'error',message:String(error.message).slice(0,512)});
      } finally {if (ownsQuery) busy = false;}
    });
  });
  await new Promise((resolve,reject) => {server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  const port = server.address().port;
  const manifest = {version:1,root,port,token};
  const temporary = `${connectionFile}.${session}.tmp`;
  try {
    await fs.mkdir(path.dirname(connectionFile), {recursive:true,mode:0o700});
    await fs.writeFile(temporary, JSON.stringify(manifest), {mode:0o600,flag:'wx'});
    await fs.rename(temporary,connectionFile);
  } catch (error) {
    server.close();
    await fs.rm(temporary,{force:true});
    throw error;
  }
  return {
    port,
    async dispose() {
      if (stopped) return;
      stopped = true;
      for (const socket of sockets) socket.destroy();
      await new Promise(resolve=>server.close(resolve));
    },
  };
}

module.exports = {startBridge};
