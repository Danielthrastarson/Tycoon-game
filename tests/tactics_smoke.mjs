// Playwright gameplay checks for the new Rust/WASM tactical siege.
// Tests real mouse/touch controls on phones, foldable and desktop.
// Do not infer a correct simulation from a visible button alone:
// deterministic Rust tests cover movement, commands, replay and audits.
import {chromium} from 'playwright';
import {PNG} from 'pngjs';
import fs from 'node:fs';
import path from 'node:path';

const out=path.resolve('playtest-artifacts');
fs.mkdirSync(out,{recursive:true});
const browser=await chromium.launch({
  ...(process.env.GENESIS_CHROME_BIN
    ?{executablePath:process.env.GENESIS_CHROME_BIN}:{}),
  headless:true,
  args:['--use-gl=angle','--use-angle=swiftshader','--enable-webgl','--no-sandbox']
});
const cases=[
  {name:'tactics-android',w:393,h:852,mobile:true,dpr:3},
  {name:'tactics-small',w:360,h:640,mobile:true,dpr:3},
  {name:'tactics-fold',w:768,h:968,mobile:true,dpr:2.5},
  {name:'tactics-desktop',w:1200,h:800,mobile:false,dpr:1}
];
const issues=[];
const screenshot=async page=>PNG.sync.read(await page.screenshot({scale:'css'}));
const color=async(page,p)=>{
  const im=await screenshot(page);
  const at=(Math.floor(p.y)*im.width+Math.floor(p.x))*4;
  return [...im.data.subarray(at,at+3)];
};
const diff=(a,b,region,step=4)=>{
  let changed=0;
  for(let y=Math.ceil(region.y+7);y<Math.floor(region.y+region.h-7);y+=step){
    for(let x=Math.ceil(region.x+7);x<Math.floor(region.x+region.w-7);x+=step){
      const i=(a.width*y+x)*4;
      const d=Math.abs(a.data[i]-b.data[i])+
        Math.abs(a.data[i+1]-b.data[i+1])+
        Math.abs(a.data[i+2]-b.data[i+2]);
      if(d>54)changed++;
    }
  }
  return changed;
};

for(const d of cases){
  const ctx=await browser.newContext({
    viewport:{width:d.w,height:d.h},
    isMobile:d.mobile,hasTouch:d.mobile,deviceScaleFactor:d.dpr
  });
  const page=await ctx.newPage();
  page.on('pageerror',error=>issues.push(d.name+': '+error.message));
  page.on('requestfailed',r=>issues.push(d.name+' '+r.url()+' '+r.failure()?.errorText));
  const response=await page.goto('http://127.0.0.1:8000/tactics.html',
    {waitUntil:'load',timeout:30000});
  if(!response||response.status()!==200)throw Error(d.name+': missing tactical HTML');
  await page.waitForTimeout(1900);
  const meta=await page.locator('canvas').evaluate(el=>{
    const rect=el.getBoundingClientRect();
    return {width:el.width,height:el.height,w:rect.width,h:rect.height};
  });
  if(meta.width<120||meta.height<120 ||
     Math.abs(meta.width-meta.w)>2||Math.abs(meta.height-meta.h)>2)
    throw Error(d.name+': touch coordinates or canvas failed '+JSON.stringify(meta));
  if(await page.locator('#error').evaluate(e=>getComputedStyle(e).display!=='none'))
    throw Error(d.name+': WebAssembly startup error');

  const wide=d.w>=d.h*1.22;
  const controlWidth=wide?Math.min(345,Math.max(280,d.w*.31)):d.w-20;
  const controlX=wide?d.w-controlWidth-10:10;
  const start=wide?142:Math.max(426,d.h-207);
  const board=wide
    ?{x:12,y:111,w:Math.max(170,controlX-24),h:Math.max(140,d.h-129)}
    :{x:10,y:108,w:Math.max(160,d.w-20),h:Math.max(155,start-160)};
  const squadWidth=(controlWidth-7)/2;
  const commandWidth=(controlWidth-21)/4;
  const utilityWidth=(controlWidth-28)/5;
  const squad=i=>({x:controlX+i*(squadWidth+7)+squadWidth*.5,y:start+21});
  const command=i=>({x:controlX+i*(commandWidth+7)+commandWidth*.5,y:start+49+23.5});
  const util=i=>({x:controlX+i*(utilityWidth+7)+utilityWidth*.5,y:start+103+19});
  const tap=async p=>{
    if(d.mobile)await page.touchscreen.tap(p.x,p.y);
    else await page.mouse.click(p.x,p.y);
    await page.waitForTimeout(155);
  };
  const boardPos=(x,y)=>{
    const height=9, width=Math.min(15,height*board.w/board.h);
    const left=Math.max(0,Math.min(15-width,7-width*.5));
    const top=Math.max(0,Math.min(9-height,4-height*.5));
    return {
      x:board.x+(x+.5-left)/width*board.w,
      y:board.y+(y+.5-top)/height*board.h,
    };
  };

  const first=await screenshot(page);
  await page.screenshot({path:path.join(out,d.name+'-initial.png'),scale:'css'});
  const colors=new Set();
  for(let y=Math.ceil(board.y+8);y<Math.floor(board.y+board.h-8);y+=5)
    for(let x=Math.ceil(board.x+8);x<Math.floor(board.x+board.w-8);x+=5){
      const i=(first.width*y+x)*4;
      colors.add([first.data[i],first.data[i+1],first.data[i+2]].join(','));
    }
  // The new atlas must have significantly more natural tonal variation
  // than a handful of flat colored tiles. This also catches missing textures.
  if(colors.size<185)
    throw Error(d.name+': painterly landscape missing detailed terrain ('+colors.size+' sampled colors)');
  const utilitySample={x:controlX+5,y:start+103+5};
  const beforePlay=await color(page,utilitySample);
  await tap(util(0)); // start
  const afterPlay=await color(page,utilitySample);
  if(afterPlay.every((v,i)=>Math.abs(v-beforePlay[i])<12))
    throw Error(d.name+': PLAY button does not change');
  await page.waitForTimeout(1250);
  await tap(util(0)); // pause
  await tap(squad(1)); // select scout
  await tap(command(1)); // attack
  await tap(boardPos(8,3)); // issue an actual attack destination
  const afterCommand=await screenshot(page);
  if(diff(first,afterCommand,board)<55)
    throw Error(d.name+': attack command failed to cause visible map change');
  await tap(command(2)); // HOLD scout
  await tap(command(3)); // RETREAT scout
  await tap(util(3)); // zoom in
  await tap(util(4)); // zoom out
  const beforePan=await screenshot(page);
  const pauseColor=await color(page,utilitySample);
  const from={x:board.x+board.w*.67,y:board.y+board.h*.56};
  const to={x:board.x+board.w*.38,y:from.y};
  if(d.mobile){
    const cdp=await ctx.newCDPSession(page);
    const coords=(x,y)=>[{x,y,id:55,radiusX:3,radiusY:3}];
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:coords(from.x,from.y)});
    for(let t=1;t<=10;t++){
      const a=t/10;
      await cdp.send('Input.dispatchTouchEvent',{
        type:'touchMove',
        touchPoints:coords(from.x+(to.x-from.x)*a,from.y)
      });
      await page.waitForTimeout(18);
    }
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
    await cdp.detach();
  }else{
    await page.mouse.move(from.x,from.y);
    await page.mouse.down();
    await page.mouse.move(to.x,to.y,{steps:14});
    await page.mouse.up();
  }
  await page.waitForTimeout(340);
  const afterPan=await screenshot(page);
  await page.screenshot({path:path.join(out,d.name+'-after-pan.png'),scale:'css'});
  const change=diff(beforePan,afterPan,board);
  if(change<80)throw Error(d.name+': drag did not pan map '+change);
  const afterPanColor=await color(page,utilitySample);
  if(afterPanColor.some((v,i)=>Math.abs(v-pauseColor[i])>16))
    throw Error(d.name+': dragging unexpectedly resumed simulation');
  // The minimap is critical for tall portrait phones: it must recenter
  // the full-world camera, never issue an attack command.
  const miniWidth=Math.min(board.w-12,Math.max(110,Math.min(155,board.w*.32)));
  const mini={x:board.x+board.w-miniWidth-9,y:board.y+9,
    w:miniWidth,h:miniWidth*9/15};
  const beforeMini=await screenshot(page);
  await tap({x:mini.x+mini.w*.10,y:mini.y+mini.h*.55});
  const afterMini=await screenshot(page);
  const minimapChanged=diff(beforeMini,afterMini,board);
  if(minimapChanged<65)
    throw Error(d.name+': tappable minimap did not recenter camera');
  await tap(util(2)); // new map / update the SAME GPU texture
  await page.waitForTimeout(350);
  await page.screenshot({path:path.join(out,d.name+'-new-map.png'),scale:'css'});
  const changedMap=diff(first,await screenshot(page),board);
  if(changedMap<125)
    throw Error(d.name+': new map did not visibly change the landscape');
  // Repeated restarts must not produce blank, crashed or stale WebGL scenes.
  for(let i=0;i<3;i++){
    await tap(util(2));
    await page.waitForTimeout(140);
  }
  const afterRestarts=await screenshot(page);
  const terrainSample=(x,y)=>{
    const at=(Math.floor(y)*afterRestarts.width+Math.floor(x))*4;
    return [...afterRestarts.data.subarray(at,at+3)];
  };
  if(terrainSample(board.x+20,board.y+board.h*.6).every(v=>v<25))
    throw Error(d.name+': terrain turned blank after repeated atlas updates');
  const err=await page.locator('#error').evaluate(el=>
    getComputedStyle(el).display!=='none');
  if(err)throw Error(d.name+': startup error shown after gameplay');
  console.log(JSON.stringify({device:d.name,canvas:meta,colors:colors.size,
    panChangedPixels:change,minimapChangedPixels:minimapChanged,
    changedMapPixels:changedMap,
    orders:'attack/hold/retreat',
    controls:'play/pause/zoom/new-map/touch',result:'PASS'}));
  await ctx.close();
}
await browser.close();
if(issues.length)throw Error(issues.join('\n'));
console.log('PASS: V0.4.4 real touch/mouse tactical siege playtest');
