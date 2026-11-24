const fs = require('node:fs/promises');
const net = require('node:net');
const crypto = require('node:crypto');
const path = require('node:path');
const {savedSource} = require('./editor-source.cjs');

const FRAME = 64*1024;
const hex = value=>typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
const keys = (object,allowed)=>object && typeof object === 'object' && !Array.isArray(object)
  && Object.keys(object).every(key=>allowed.includes(key));
const issue = (code,message)=>Object.assign(new Error(message),{code});
function message(error) {
  let text = String(error.message);
  while (Buffer.byteLength(text) > 512) text = text.slice(0,-1);
  return text;
}
function validate(request,{root,session,token,javaArchives}) {
  if (request?.protocol !== 'vizzi-origin' || request.version !== 1 || request.root !== root
      || request.session !== session || !hex(request.token)
      || !crypto.timingSafeEqual(Buffer.from(request.token),Buffer.from(token))) {
    throw issue('invalid_connection','Invalid originating editor connection');
  }
  if (!keys(request,['protocol','version','root','session','token','request','path','line','source','expires_at_ms','providerUri','archiveBinary','archiveCapture'])
      || !hex(request.request) || typeof request.path !== 'string' || !path.isAbsolute(request.path)
      || Buffer.byteLength(request.path) > 4096 || !Number.isSafeInteger(request.line) || request.line < 1
      || !keys(request.source,['bytes','sha256']) || !Number.isSafeInteger(request.source.bytes)
      || request.source.bytes < 0 || request.source.bytes > 16*1024*1024 || !hex(request.source.sha256)
      || !Number.isSafeInteger(request.expires_at_ms)) {
    throw issue('invalid_request','Invalid verified editor request');
  }
  if ((request.providerUri !== undefined || request.archiveBinary !== undefined || request.archiveCapture !== undefined)
      && (!javaArchives || typeof request.providerUri !== 'string' || Buffer.byteLength(request.providerUri) > 16*1024
        || typeof request.archiveBinary !== 'string' || !path.isAbsolute(request.archiveBinary)
        || Buffer.byteLength(request.archiveBinary) > 4096 || !hex(request.archiveCapture))) {
    throw issue('invalid_request','Invalid verified Java archive request');
  }
  if (request.expires_at_ms <= Date.now()) throw issue('expired','Originating editor request expired');
}

async function startOriginEditor({root,storage,name,open,javaArchives=false}) {
  if (typeof name !== 'string' || !name || Buffer.byteLength(name) > 64 || /[\x00-\x1f\x7f]/.test(name)) {
    throw new Error('Invalid originating editor name');
  }
  await fs.mkdir(storage,{recursive:true,mode:0o700});
  storage = await fs.realpath(storage);
  const session = crypto.randomBytes(32).toString('hex'), token = crypto.randomBytes(32).toString('hex');
  const file = path.join(storage,session+'.json'), relative = path.relative(root,file);
  if (relative === '' || relative !== '..' && !relative.startsWith('..'+path.sep) && !path.isAbsolute(relative)) {
    throw new Error('Originating editor storage must be outside captured source');
  }
  let stopped = false, busy = false, disposed;
  const sockets = new Set(), controllers = new Set();
  const server = net.createServer(socket=>{
    if (stopped || sockets.size >= 4) {socket.destroy();return;}
    sockets.add(socket);
    const controller = new AbortController();
    controllers.add(controller);
    const timer = setTimeout(()=>{controller.abort();socket.destroy();},12000);
    socket.on('error',()=>{});
    socket.on('end',()=>controller.abort());
    socket.once('close',()=>{
      clearTimeout(timer);controller.abort();controllers.delete(controller);sockets.delete(socket);
    });
    let chunks = [], length = 0, received = false;
    socket.on('data',async bytes=>{
      if (received) {socket.destroy();return;}
      length += bytes.length;
      if (length > FRAME) {socket.destroy();return;}
      chunks.push(bytes);
      if (!bytes.includes(10)) return;
      received = true;
      const frame = Buffer.concat(chunks);
      chunks = [];
      let request, owned = false, executionTimer, abort;
      function send(value) {
        if (!socket.destroyed && !socket.writableEnded) {
          socket.end(JSON.stringify({protocol:'vizzi-origin',version:1,session,
            request:hex(request?.request) ? request.request : '',...value})+'\n');
        }
      }
      try {
        if (frame.indexOf(10) !== frame.length-1) throw issue('invalid_request','Expected one originating editor frame');
        request = JSON.parse(frame.subarray(0,-1).toString('utf8'));
        validate(request,{root,session,token,javaArchives});
        if (busy) throw issue('busy','Originating editor navigation is busy; retry');
        busy = true;
        owned = true;
        executionTimer = setTimeout(()=>controller.abort(),Math.min(8000,request.expires_at_ms-Date.now()));
        const cancelled = new Promise((_,reject)=>{
          abort = ()=>reject(issue('expired','Originating editor request expired or closed'));
          controller.signal.addEventListener('abort',abort,{once:true});
          if (controller.signal.aborted) abort();
        });
        await Promise.race([open(request,{signal:controller.signal}),cancelled]);
        if (controller.signal.aborted || stopped || socket.destroyed || Date.now() >= request.expires_at_ms) {
          throw issue('expired','Originating editor request expired or closed');
        }
        send({status:'opened'});
      } catch (error) {
        send({status:'error',code:typeof error.code === 'string' ? error.code.slice(0,64) : 'unavailable',message:message(error)});
      } finally {
        clearTimeout(executionTimer);
        if (abort) controller.signal.removeEventListener('abort',abort);
        if (owned) busy = false;
      }
    });
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  const contents = Buffer.from(JSON.stringify({protocol:'vizzi-origin',version:1,root,editor:name,
    session,port:server.address().port,token}));
  const temporary = file+'.tmp';
  async function dispose() {
    if (disposed) return disposed;
    stopped = true;
    for (const controller of controllers) controller.abort();
    for (const socket of sockets) socket.destroy();
    disposed = (async()=>{
      await new Promise(resolve=>server.close(resolve));
      try {if ((await savedSource(file,4096)).equals(contents)) await fs.unlink(file);} catch {}
    })();
    return disposed;
  }
  try {
    await fs.writeFile(temporary,contents,{flag:'wx',mode:0o600});
    await fs.rename(temporary,file);
  } catch (error) {
    await dispose();
    await fs.rm(temporary,{force:true}).catch(()=>{});
    throw error;
  }
  return {file,dispose,javaArchiveSession:javaArchives?session:undefined};
}
module.exports = {startOriginEditor};
