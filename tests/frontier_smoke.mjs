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
 const buttonBackground=()=>sample({x:railx+18,y:controlsy+8});
 const initialButton=await buttonBackground();
 await tap(btn(0)); // PAUSE
 const pausedButton=await buttonBackground();
 if(initialButton[1]===pausedButton[1] && initialButton[2]===pausedButton[2])
   throw Error(s.name+': PLAY button did not switch its visual state');
 await tap(btn(0)); // resume
 await tap(btn(2)); // recruit => selected cyan ring now at capital
 await tap(btn(5)); // attack mode
 // Portrait viewport fills the available board; its world view is
 // deliberately cropped horizontally. The same transform is used in Rust.
 const board={
   x:wide?12:9,y:82,
   w:wide?Math.max(100,railx-24):Math.max(100,s.width-18),
   h:wide?Math.max(100,s.height-100):Math.max(100,controlsy-146)
 };
 const viewH=52,viewW=Math.min(72,viewH*board.w/board.h);
 const viewX=Math.max(0,Math.min(72-viewW,26-viewW*.5));
 const viewY=0;
 const bx=board.x,by=board.y;
 const map=(x,y)=>({
   x:board.x+(x+.5-viewX)/viewW*board.w,
   y:board.y+(y+.5-viewY)/viewH*board.h
 });
 await tap(map(44,24));
 await tap(btn(4)); // hold selected army
 await tap(btn(6)); // retreat
 await tap(btn(7)); // zoom in
 await tap(btn(8)); // zoom out

 // Interactive panning regression: first pause, then zoom into the world.
 // Dragging must visibly move the camera without issuing an army order or
 // changing the PAUSE/PLAY button to running.
 await tap(btn(0));
 await tap(btn(7));
 await tap(btn(7));
 await page.waitForTimeout(420);
 const viewBefore=PNG.sync.read(await page.screenshot({scale:'css'}));
 const pausedBefore=await buttonBackground();
 const from={x:board.x+board.w*.72,y:board.y+board.h*.55};
 const to={x:board.x+board.w*.38,y:from.y};
 const drag=async()=>{
   if(s.mobile) {
     const cdp=await ctx.newCDPSession(page);
     const point=(x,y)=>[{x,y,id:77,radiusX:3,radiusY:3}];
     await cdp.send('Input.dispatchTouchEvent',{
       type:'touchStart',touchPoints:point(from.x,from.y)
     });
     for(let step=1;step<=9;step++){
       const alpha=step/9;
       await cdp.send('Input.dispatchTouchEvent',{
         type:'touchMove',touchPoints:point(
           from.x+(to.x-from.x)*alpha,from.y+(to.y-from.y)*alpha)
       });
       await page.waitForTimeout(20);
     }
     await cdp.send('Input.dispatchTouchEvent',{
       type:'touchEnd',touchPoints:[]
     });
     await cdp.detach();
   } else {
     await page.mouse.move(from.x,from.y);
     await page.mouse.down();
     await page.mouse.move(to.x,to.y,{steps:12});
     await page.mouse.up();
   }
 };
 await drag();
 await page.waitForTimeout(450);
 const viewAfter=PNG.sync.read(await page.screenshot({
   path:path.join(output,s.name+'-panned.png'),scale:'css'
 }));
 let changed=0;
 for(let y=Math.ceil(by+10);y<Math.floor(by+board.h-10);y+=3)
   for(let x=Math.ceil(bx+10);x<Math.floor(bx+board.w-10);x+=3){
     const i=(viewBefore.width*y+x)*4;
     const delta=Math.abs(viewBefore.data[i]-viewAfter.data[i])+
       Math.abs(viewBefore.data[i+1]-viewAfter.data[i+1])+
       Math.abs(viewBefore.data[i+2]-viewAfter.data[i+2]);
     if(delta>38)changed++;
   }
 if(changed<90)
   throw Error(s.name+': drag failed to pan camera; changed sampled pixels='+changed);
 const pausedAfter=await buttonBackground();
 if(pausedAfter.some((v,i)=>Math.abs(v-pausedBefore[i])>12))
   throw Error(s.name+': drag accidentally resumed the game / issued order');
 await tap(btn(1)); // new world resets
 await page.waitForTimeout(450);
 await page.screenshot({path:path.join(output,s.name+'-after.png'),scale:'css'});
 const errorVisible=await page.locator('#error').evaluate(el=>getComputedStyle(el).display!=='none');
 if(errorVisible)throw Error(s.name+': Frontier startup error displayed');
 console.log(JSON.stringify({scenario:s.name,colors:colors.size,canvas:meta,commands:'pause/play/recruit/attack/hold/retreat/zoom/pan/reset',panningPixels:changed,status:'PASS'}));
 await ctx.close();
}
await browser.close();
if(errors.length)throw Error(errors.join('\n'));
console.log('PASS: Genesis Frontier rendered and responded to touch and mouse gestures, including camera panning without unintended orders.');
