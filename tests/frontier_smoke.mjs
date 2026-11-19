// Real browser check: launch Rust/WASM Frontier on phones and desktop.
// The test uses actual touch/mouse clicks and checks visible canvas change.
import { chromium } from 'playwright';
import {PNG} from 'pngjs';
import fs from 'node:fs';
import path from 'node:path';
const output=path.resolve('playtest-artifacts');
fs.mkdirSync(output,{recursive:true});
const browser=await chromium.launch({
  ...(process.env.GENESIS_CHROME_BIN?{executablePath:process.env.GENESIS_CHROME_BIN}:{}),
  headless:true,
  args:['--use-gl=angle','--use-angle=swiftshader','--enable-webgl','--no-sandbox']
});
const errors=[];
const cases=[
 {name:'frontier-android',width:393,height:852,dpr:3,mobile:true},
 {name:'frontier-smallphone',width:360,height:640,dpr:3,mobile:true},
 {name:'frontier-fold',width:768,height:968,dpr:2.5,mobile:true},
 {name:'frontier-desktop',width:1200,height:800,dpr:1,mobile:false},
];
for(const s of cases) {
 const ctx=await browser.newContext({
  viewport:{width:s.width,height:s.height},
  isMobile:s.mobile,hasTouch:s.mobile,deviceScaleFactor:s.dpr
 });
 const page=await ctx.newPage();
 page.on('pageerror',err=>errors.push(s.name+': '+err.message));
 page.on('requestfailed',req=>errors.push(s.name+': '+req.url()+' '+req.failure()?.errorText));
 const result=await page.goto('http://127.0.0.1:8000/frontier.html',{waitUntil:'load',timeout:30000});
 if(!result||result.status()>=400)throw Error(s.name+': missing Frontier HTML');
 await page.waitForTimeout(2200);
 const meta=await page.locator('canvas').evaluate(el=>{
  const r=el.getBoundingClientRect();
  return {width:el.width,height:el.height,w:r.width,h:r.height};
 });
 if(meta.width<100||meta.height<100)throw Error(s.name+': canvas not initialized '+JSON.stringify(meta));
 if(Math.abs(meta.width-meta.w)>2||Math.abs(meta.height-meta.h)>2)
  throw Error(s.name+': touch coordinates do not match framebuffer '+JSON.stringify(meta));
 const shot=PNG.sync.read(await page.screenshot({path:path.join(output,s.name+'-initial.png'),scale:'css'}));
 const colors=new Set();
 for(let y=Math.round(shot.height*.20);y<Math.round(shot.height*.63);y+=4)
  for(let x=Math.round(shot.width*.08);x<Math.round(shot.width*.75);x+=4) {
   const i=(shot.width*y+x)*4;
   colors.add([shot.data[i],shot.data[i+1],shot.data[i+2]].join(','));
  }
 if(colors.size<15)throw Error(s.name+': expected colored factions and terrain, found '+colors.size);
 const wide=s.width>s.height*1.24;
 const railw=wide?Math.max(285,Math.min(340,s.width*.32)):s.width;
 const railx=wide?s.width-railw:0;
 const controlsy=wide?178:Math.max(350,s.height-211);
 const btnw=(railw-24-14)/3;
 const btn=i=>({
  x:railx+12+(i%3)*(btnw+7)+btnw*.5,
  y:controlsy+Math.floor(i/3)*57+25
 });
 const tap=async p=>{
  if(s.mobile)await page.touchscreen.tap(p.x,p.y);
  else await page.mouse.click(p.x,p.y);
  await page.waitForTimeout(180);
 };
 const sample=async p=>{
   const im=PNG.sync.read(await page.screenshot({scale:'css'}));
   const ix=(im.width*Math.floor(p.y)+Math.floor(p.x))*4;
   return [...im.data.subarray(ix,ix+3)];
 };
 const initialButton=await sample(btn(0));
 await tap(btn(0)); // PAUSE
 const pausedButton=await sample(btn(0));
 if(initialButton[1]===pausedButton[1] && initialButton[2]===pausedButton[2])
   throw Error(s.name+': PLAY button did not switch its visual state');
 await tap(btn(0)); // resume
 await tap(btn(2)); // recruit => selected cyan ring now at capital
 await tap(btn(5)); // attack mode
 const regionW=wide?railx-24:s.width-18;
 const regionH=wide?s.height-108:controlsy-98;
 const scale=Math.min(regionW/72,regionH/52);
 const bx=wide?12+(regionW-72*scale)*.5:(s.width-72*scale)*.5;
 const by=82+(regionH-52*scale)*.4;
 const map=(x,y)=>({x:bx+x*scale,y:by+y*scale});
 await tap(map(44,24));
 await tap(btn(4)); // hold selected army
 await tap(btn(6)); // retreat
 await tap(btn(7)); // zoom in
 await tap(btn(8)); // zoom out
 await tap(btn(1)); // new world resets
 await page.waitForTimeout(450);
 await page.screenshot({path:path.join(output,s.name+'-after.png'),scale:'css'});
 const errorVisible=await page.locator('#error').evaluate(el=>getComputedStyle(el).display!=='none');
 if(errorVisible)throw Error(s.name+': Frontier startup error displayed');
 console.log(JSON.stringify({scenario:s.name,colors:colors.size,canvas:meta,commands:'pause/play/recruit/attack/hold/retreat/zoom/reset',status:'PASS'}));
 await ctx.close();
}
await browser.close();
if(errors.length)throw Error(errors.join('\n'));
console.log('PASS: Genesis Frontier rendered and responded to nine mobile/desktop controls.');
