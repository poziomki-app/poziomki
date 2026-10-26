import http from 'node:http';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { basename, join, extname } from 'node:path';
import { homedir } from 'node:os';
const root=join(process.env.SHOWCASE_DIR||join(homedir(),'.cache/poziomki-showcase'),'assets');
const server=http.createServer(async(req,res)=>{
 if(req.url.startsWith('/api/v1/uploads/')){
  const file=join(root,basename(new URL(req.url,'http://localhost').pathname));
  try{const info=await stat(file);res.writeHead(200,{'Content-Type':({'.jpg':'image/jpeg','.jpeg':'image/jpeg','.avif':'image/avif','.png':'image/png'})[extname(file)]||'application/octet-stream','Content-Length':info.size});createReadStream(file).pipe(res)}catch{res.writeHead(404).end()}
  return;
 }
 const upstream=http.request({host:'127.0.0.1',port:5157,path:req.url,method:req.method,headers:req.headers},response=>{res.writeHead(response.statusCode,response.headers);response.pipe(res)});
 upstream.on('error',()=>res.writeHead(502).end());req.pipe(upstream);
});
server.on('upgrade',(req,socket,head)=>{
 const upstream=http.request({host:'127.0.0.1',port:5157,path:req.url,method:req.method,headers:req.headers});
 upstream.on('upgrade',(response,peer,peerHead)=>{
  socket.write(`HTTP/1.1 ${response.statusCode} Switching Protocols\r\n`+Object.entries(response.headers).map(([k,v])=>`${k}: ${v}\r\n`).join('')+'\r\n');
  if(peerHead.length)socket.write(peerHead);if(head.length)peer.write(head);peer.pipe(socket);socket.pipe(peer);
 });
 upstream.on('error',()=>socket.destroy());upstream.end();
});
server.listen(5155,'127.0.0.1',()=>console.log('Showcase proxy listening on 127.0.0.1:5155.'));
