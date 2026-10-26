import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, writeFile, copyFile, readFile, readdir } from 'node:fs/promises';
const base='http://127.0.0.1:5155/api/v1';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
const source=dirname(fileURLToPath(import.meta.url));
const directory=process.env.SHOWCASE_DIR||join(homedir(),'.cache/poziomki-showcase');
await mkdir(directory,{recursive:true});
const password='ShowcaseLocal2026!';
const email='ola@example.com';
const q=v=>v==null?'NULL':`'${String(v).replaceAll("'","''")}'`;
const sql=command=>execFileSync('psql',['-h','127.0.0.1','-p','55435','-U','showcase','-d','poziomki_showcase','-v','ON_ERROR_STOP=1','-At'],{encoding:'utf8',input:command}).trim();
const response=await fetch(base+'/auth/sign-up/email',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email,password,name:'Ola'})});
console.log('Local registration:',response.status);
if(![200,201,409].includes(response.status))throw new Error(await response.text());
sql(`UPDATE users SET email_verified_at=now() WHERE email=${q(email)}`);
const people=JSON.parse(await readFile(join(source,'profiles.json'),'utf8')).map(p=>[p.name,p.program,p.bio,p.tags,p.photo]);
const colors=[['#073A43','#151F2D'],['#183E46','#26252F'],['#253D2F','#172329'],['#382A40','#182D34'],['#413326','#182D35'],['#2C2940','#1C3335']];
const ids=people.map(()=>randomUUID());
let commands='BEGIN;';
for(let i=0;i<people.length;i++){
 const [name,program,bio,tags]=people[i];const userEmail=i===0?email:name.toLowerCase()+'@example.com';
 if(i)commands+=`INSERT INTO users(pid,email,password,api_key,name,email_verified_at) SELECT ${q(randomUUID())},${q(userEmail)},password,${q('demo-'+randomUUID())},${q(name)},now() FROM users WHERE email=${q(email)};`;
 commands+=`INSERT INTO profiles(id,user_id,name,bio,profile_picture,images,program,gradient_start,gradient_end) SELECT ${q(ids[i])},id,${q(name)},${q(bio)},${q(people[i][4]+'.jpg')},'[]',${q(program)},${q(colors[i%colors.length][0])},${q(colors[i%colors.length][1])} FROM users WHERE email=${q(userEmail)};`;
 for(const tag of tags){
  commands+=`INSERT INTO tags(id,name,scope) SELECT gen_random_uuid(),${q(tag)},'interest' WHERE NOT EXISTS(SELECT 1 FROM tags WHERE name=${q(tag)} AND scope='interest');`;
  commands+=`INSERT INTO profile_tags(profile_id,tag_id) SELECT ${q(ids[i])},id FROM tags WHERE name=${q(tag)} AND scope='interest' LIMIT 1;`;
 }
}
const events=JSON.parse(await readFile(join(source,'events.json'),'utf8'));
const eventIds=[];
for(let i=0;i<events.length;i++){
 const [title,category,location,cover,day,lat,lon,description]=events[i];const id=randomUUID(),conversation=randomUUID();eventIds.push(id);
 const start=new Date();start.setDate(start.getDate()+day);start.setHours(17,0,0,0);
 commands+=`INSERT INTO events(id,title,description,cover_image,location,starts_at,ends_at,creator_id,latitude,longitude,max_attendees,requires_approval,category,is_featured) VALUES(${q(id)},${q(title)},${q(description)},${q(cover)},${q(location)},${q(start.toISOString())},${q(new Date(start.getTime()+7200000).toISOString())},${q(ids[(i+1)%people.length])},${lat},${lon},12,false,${q(category)},${i===0});`;
 commands+=`INSERT INTO conversations(id,kind,title,event_id) VALUES(${q(conversation)},'event',${q(title)},${q(id)});UPDATE events SET conversation_id=${q(conversation)} WHERE id=${q(id)};`;
 for(let j=0;j<7-i%3;j++){
  commands+=`INSERT INTO event_attendees(event_id,profile_id,status) VALUES(${q(id)},${q(ids[j])},'going');INSERT INTO conversation_members(conversation_id,user_id) SELECT ${q(conversation)},user_id FROM profiles WHERE id=${q(ids[j])};`;
 }
 const chats=i===0?[[2,'Będę o 17 pod mostem, od strony plaży.'],[1,'Jadę z Mokotowa. Ktoś po drodze?'],[0,'Ja! Mogę dołączyć przy Łazienkowskim.'],[2,'To zbierzemy się tam i podjedziemy razem.']]:[[i+1,'Do zobaczenia! Fajnie, że udało się zebrać ekipę.']];
 for(let j=0;j<chats.length;j++)commands+=`INSERT INTO messages(conversation_id,sender_id,body,created_at) SELECT ${q(conversation)},user_id,${q(chats[j][1])},now()-interval '${30-j*4+i*60} minutes' FROM profiles WHERE id=${q(ids[chats[j][0]])};`;
}
for(let i=1;i<5;i++){
 const id=randomUUID();
 commands+=`INSERT INTO conversations(id,kind,user_low_id,user_high_id) SELECT ${q(id)},'dm',least(a.user_id,b.user_id),greatest(a.user_id,b.user_id) FROM profiles a,profiles b WHERE a.id=${q(ids[0])} AND b.id=${q(ids[i])};INSERT INTO conversation_members(conversation_id,user_id) SELECT ${q(id)},user_id FROM profiles WHERE id IN (${q(ids[0])},${q(ids[i])});`;
 const chat=i===1?[[1,'Będziesz jutro na rowerach?'],[0,'Tak, tylko jadę spokojnie.'],[1,'To spotkajmy się wcześniej pod mostem?'],[0,'O 17? Mam szary rower.'],[1,'Pasuje. Do jutra!']]:[[i,['','', 'Bierzesz aparat jutro?','Masz chwilę na herbatę po zajęciach?','Jedziesz nad Wisłę w weekend?'][i]]];
 for(let j=0;j<chat.length;j++)commands+=`INSERT INTO messages(conversation_id,sender_id,body,created_at) SELECT ${q(id)},user_id,${q(chat[j][1])},now()-interval '${20-j*3+i*7} minutes' FROM profiles WHERE id=${q(ids[chat[j][0]])};`;
}
commands+='COMMIT;';sql(commands);
await writeFile(directory+'/seed.sql',commands);
await writeFile(directory+'/credentials.json',JSON.stringify({email,password,profileIds:ids,eventIds},null,2),{mode:0o600});
await mkdir(directory+'/assets',{recursive:true});
for(const file of await readdir(join(source,'photos')))if(file.endsWith('.jpg'))await copyFile(join(source,'photos',file),join(directory,'assets',file));
console.log('Created 12 fictional profiles, 6 events and 10 conversations in local showcase database.');
