import http from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve, extname, sep } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const directory=dirname(fileURLToPath(import.meta.url));
const root=resolve(directory,'..');
const fonts=join(root,'mobile/app-ui/src/commonMain/composeResources/font');
const selected=process.argv[2]||'all';
if(!['all','google','apple','preview'].includes(selected))throw new Error('Use all, google, apple or preview.');
const server=http.createServer(async(req,res)=>{
 const path=new URL(req.url,'http://localhost').pathname;
 const base=path.startsWith('/fonts/')?fonts:directory;
 const file=resolve(base,'.'+(base===fonts?path.slice(6):path));
 if(file!==base&&!file.startsWith(base+sep)){res.writeHead(403).end();return}
 try{const body=await readFile(file);res.writeHead(200,{'Content-Type':({'.html':'text/html; charset=utf-8','.svg':'image/svg+xml','.png':'image/png','.ttf':'font/ttf'})[extname(file)]||'application/octet-stream'});res.end(body)}catch{res.writeHead(404).end()}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const url=`http://127.0.0.1:${server.address().port}/showcase.html`;
const env={...process.env,AGENT_BROWSER_SESSION:'poziomki-showcase-export'};
const execute=promisify(execFile);
async function browser(...args){return (await execute('agent-browser',args,{env,encoding:'utf8',timeout:60000})).stdout}
async function render(platform,index,width,height,kind,target){
 await mkdir(dirname(target),{recursive:true});
 await browser('set','viewport',String(width),String(height));
 await browser('open',`${url}?platform=${platform}&scene=${index}&kind=${kind}`);
 await browser('eval','Promise.all([...document.images].map(i=>i.decode())).then(()=>document.fonts.ready).then(()=>true)');
 await browser('screenshot',target);
 console.log(target.replace(root+'/',''));
}
try{
 if(selected==='all'||selected==='google'||selected==='preview'){
  const count=selected==='preview'?1:6;
  for(let i=0;i<count;i++)await render('android',i,1080,2160,'phone',join(directory,'google/pl-PL/images/phoneScreenshots',`${String(i+1).padStart(2,'0')}.png`));
  await render('android',0,1024,500,'feature',join(directory,'google/pl-PL/images/featureGraphic/01.png'));
  if(selected!=='preview'){
   for(const kind of ['sevenInchScreenshots','tenInchScreenshots']){
    for(let i=0;i<2;i++)await render('android',i,1280,2048,'tablet',join(directory,'google/pl-PL/images',kind,`${String(i+1).padStart(2,'0')}.png`));
   }
  }
 }
 if(selected==='all'||selected==='apple'){
  for(let i=0;i<4;i++)await render('ios',i,1284,2778,'phone',join(directory,'apple/screenshots/pl-PL',`iphone65-${String(i+1).padStart(2,'0')}.png`));
  for(let i=0;i<2;i++)await render('ios',i,2048,2732,'tablet',join(directory,'apple/screenshots/pl-PL',`ipad129-${String(i+1).padStart(2,'0')}.png`));
 }
}finally{
 try{await browser('close')}finally{server.close()}
}
