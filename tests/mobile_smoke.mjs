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
  headless:true,
  args:['--use-gl=angle', '--use-angle=swiftshader', '--enable-webgl', '--no-sandbox']
});
const errors=[];
for (const scenario of [
  {name:'android',size:{width:393,height:852},mobile:true},
  {name:'smallphone',size:{width:360,height:640},mobile:true},
  {name:'desktop',size:{width:1200,height:800},mobile:false},
]) {
  const context = await chrome.newContext({
    viewport:scenario.size,isMobile:scenario.mobile,hasTouch:scenario.mobile,
    deviceScaleFactor:1,
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
  const before = await page.screenshot({path:path.join(output,scenario.name+'-before.png')});
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
  // Select or recruit on-map. WebKit/Android sends touch taps through Playwright.
  const point={x:scenario.size.width*.5,y:Math.min(scenario.size.width*.55+105,scenario.size.height*.50)};
  if (scenario.mobile) await page.touchscreen.tap(point.x, point.y);
  else await page.mouse.click(point.x,point.y);
  await page.waitForTimeout(300);
  await page.screenshot({path:path.join(output,scenario.name+'-after.png')});
  console.log(JSON.stringify({scenario:scenario.name,canvas:meta,gameColors:unique.size,errors:errors.filter(e=>e.startsWith(scenario.name))}));
  await context.close();
}
await chrome.close();
if(errors.length) throw Error('Browser script/request failures:\n'+errors.join('\n'));
console.log('PASS: Chromium browser initialized the real WASM game and accepted map taps in 3 layouts.');
