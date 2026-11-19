// Browser-level mobile playtest. This loads the real Rust/WebAssembly build,
// uses Chromium touch inputs, captures screenshots, and fails on a blank canvas.
// No mocked simulation or separate JS physics.
import { chromium } from 'playwright';
import { PNG } from 'pngjs';
import fs from 'node:fs';
import path from 'node:path';

const output = path.resolve('playtest-artifacts');
fs.mkdirSync(output, {recursive:true});
const chrome = await chromium.launch({
  ...(process.env.GENESIS_CHROME_BIN ? {executablePath:process.env.GENESIS_CHROME_BIN} : {}),
  headless:true,
  args:['--use-gl=angle', '--use-angle=swiftshader', '--enable-webgl', '--no-sandbox']
});
const errors=[];
for (const scenario of [
  {name:'android-dpr3',size:{width:393,height:852},mobile:true,dpr:3},
  {name:'smallphone-dpr3',size:{width:360,height:640},mobile:true,dpr:3},
  {name:'fold-cover',size:{width:344,height:882},mobile:true,dpr:3},
  {name:'fold-unfolded',size:{width:768,height:968},mobile:true,dpr:2.5},
  {name:'desktop',size:{width:1200,height:800},mobile:false,dpr:1},
]) {
  const context = await chrome.newContext({
    viewport:scenario.size,isMobile:scenario.mobile,hasTouch:scenario.mobile,
    deviceScaleFactor:scenario.dpr,
  });
  const page=await context.newPage();
  page.on('pageerror', err=>errors.push(scenario.name+': '+err.message));
  page.on('requestfailed',req=>errors.push(scenario.name+': failed '+req.url()+' '+req.failure()?.errorText));
  const result = await page.goto('http://127.0.0.1:8000/',{waitUntil:'load',timeout:30000});
  if (!result || result.status() >= 400) throw Error(scenario.name+': HTTP '+result?.status());
  await page.waitForTimeout(3000);
  const canvas=await page.locator('canvas');
  const meta=await canvas.evaluate(el=>({
    width:el.width,height:el.height,
    rect:el.getBoundingClientRect().toJSON()
  }));
  if (meta.width < 100 || meta.height < 100) throw Error(scenario.name+': canvas not initialized: '+JSON.stringify(meta));
  // Physical Android screens are 2–4× DPI. The renderer MUST render in
  // CSS pixels, or buttons shrink below touch-target size on actual phones.
  if (Math.abs(meta.width-meta.rect.width)>2 || Math.abs(meta.height-meta.rect.height)>2)
    throw Error(scenario.name+': high-DPI framebuffer/input coordinate mismatch: '+JSON.stringify(meta));
  const before = await page.screenshot({path:path.join(output,scenario.name+'-before.png'),scale:'css'});
  const png=PNG.sync.read(before);
  const unique=new Set();
  // Sample only the central game-map area, not the browser overlays.
  for(let y=Math.floor(png.height*.2);y<Math.floor(png.height*.62);y+=7){
    for(let x=Math.floor(png.width*.13);x<Math.floor(png.width*.87);x+=7){
      const i=(png.width*y+x)*4;
      unique.add([png.data[i],png.data[i+1],png.data[i+2]].join(','));
    }
  }
  if (unique.size < 5) throw Error(scenario.name+': blank or unrendered WebAssembly canvas, '+unique.size+' colors: '+JSON.stringify(errors));
  // Real interactive playtest: tap the world to recruit existing people.
  const wide=scenario.size.width>scenario.size.height*1.25;
  const panelW=wide?Math.max(285,Math.min(370,scenario.size.width*.35)):scenario.size.width;
  const panelX=wide?scenario.size.width-panelW:0;
  const controlsY=wide?172:scenario.size.height-206;
  const boardSide=Math.max(86,Math.min(
    wide?panelX-22:scenario.size.width-22,
    wide?scenario.size.height-112:controlsY-148));
  const boardX=wide?12:(scenario.size.width-boardSide)*.5;
  const boardY=87;
  const point={x:boardX+boardSide*.5,y:boardY+boardSide*.5};
  const tap=async ({x,y})=>{
    if(scenario.mobile) await page.touchscreen.tap(x,y);
    else await page.mouse.click(x,y);
    await page.waitForTimeout(180);
  };
  const buttonPoint=i=>({
    x:panelX+12+(i%3)*((panelW-24-14)/3+7)+((panelW-24-14)/3)/2,
    y:controlsY+Math.floor(i/3)*60+26
  });
  const cyanCount=png=>{
    let count=0;
    for(let y=Math.ceil(boardY+5);y<Math.floor(boardY+boardSide-5);y++){
      for(let x=Math.ceil(boardX+5);x<Math.floor(boardX+boardSide-5);x++){
        const i=(png.width*y+x)*4;
        const [r,g,b]=png.data.subarray(i,i+3);
        if(b>230 && g>195 && r<130) count++;
      }
    }
    return count;
  };
  const beforeCyan=cyanCount(png);
  await tap(point);
  const recruited=PNG.sync.read(await page.screenshot({path:path.join(output,scenario.name+'-recruited.png'),scale:'css'}));
  const afterCyan=cyanCount(recruited);
  if(afterCyan < beforeCyan+15) throw Error(scenario.name+': tap did not recruit visible cyan soldiers ('+beforeCyan+' -> '+afterCyan+')');
  // The main command menu remains visible. Tap MOVE, set a destination,
  // select HOLD, set its destination, retreat, then disband.
  // The prior test only tapped the MAP. Reproduce the actual reported bug:
  // press canvas-drawn buttons on a 3x-DPI phone and verify actual state change.
  const buttonBg=async()=> {
    const shot=PNG.sync.read(await page.screenshot({scale:'css'}));
    const x=Math.round(panelX+18), y=Math.round(controlsY+8);
    const i=(shot.width*y+x)*4;
    return [shot.data[i],shot.data[i+1],shot.data[i+2]];
  };
  const pausedColor=await buttonBg();
  await tap(buttonPoint(0)); // PLAY/PAUSE toggle
  const playingColor=await buttonBg();
  if(Math.abs(playingColor[1]-pausedColor[1])<20)
    throw Error(scenario.name+': PLAY/PAUSE button did not switch state: '+pausedColor+' => '+playingColor);
  await tap(buttonPoint(0)); // toggle back
  const pausedAgain=await buttonBg();
  if(Math.abs(pausedAgain[1]-pausedColor[1])>15)
    throw Error(scenario.name+': PAUSE button did not switch back: '+pausedColor+' => '+pausedAgain);
  await tap(buttonPoint(8)); // OPTIONS
  const optionsPng=PNG.sync.read(await page.screenshot({scale:'css'}));
  const compareRegion=(a,b)=>{
    let changed=0;
    for(let y=Math.floor(controlsY+7);y<Math.floor(controlsY+40);y+=2)
      for(let x=Math.floor(panelX+17);x<Math.floor(panelX+buttonWidth-8);x+=2){
        const i=(a.width*y+x)*4;
        if(Math.abs(a.data[i]-b.data[i])+Math.abs(a.data[i+1]-b.data[i+1])+Math.abs(a.data[i+2]-b.data[i+2])>28) changed++;
      }
    return changed;
  };
  // Options page changes the first button to BACK, and its text pixels.
  // Note normal screenshot is captured after options; use the pre-OPTIONS
  // recruited snapshot only for a rough page-difference check.
  if(compareRegion(recruited,optionsPng)<25)
    throw Error(scenario.name+': OPTIONS button did not change visible menu');
  await tap(buttonPoint(0)); // BACK
  await tap(buttonPoint(3)); // MOVE
  await tap({x:boardX+boardSide*.78,y:boardY+boardSide*.70});
  await tap(buttonPoint(4)); // HOLD
  await tap({x:boardX+boardSide*.30,y:boardY+boardSide*.40});
  await tap(buttonPoint(5)); // RETREAT
  await tap(buttonPoint(6)); // DISBAND
  const disbanded=PNG.sync.read(await page.screenshot({path:path.join(output,scenario.name+'-after.png'),scale:'css'}));
  const finalCyan=cyanCount(disbanded);
  if(finalCyan > Math.max(10,beforeCyan+5)) throw Error(scenario.name+': DISBAND did not remove the soldier highlight ('+finalCyan+')');
  console.log(JSON.stringify({scenario:scenario.name,dpr:scenario.dpr,canvas:meta,gameColors:unique.size,
    recruitedCyan:afterCyan,disbandCyan:finalCyan,
    errors:errors.filter(e=>e.startsWith(scenario.name))}));
  await context.close();
}
await chrome.close();
if(errors.length) throw Error('Browser script/request failures:\n'+errors.join('\n'));
console.log('PASS: Chromium browser initialized the real WASM game and accepted map taps in 3 layouts.');
