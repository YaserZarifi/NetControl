import { mkdirSync, writeFileSync } from 'node:fs';
import { Buffer } from 'node:buffer';
// A 32-bit Windows icon with a compact network waveform.
const size = 32;
const pixels = Buffer.alloc(size * size * 4);
const line = [[5,17],[11,17],[14,8],[18,25],[21,15],[27,15]];
for (let y=0;y<size;y++) for(let x=0;x<size;x++) {
  const i=((size-1-y)*size+x)*4;
  const inside=x>=2&&x<30&&y>=2&&y<30;
  let stroke=false;
  for(let n=1;n<line.length;n++) {
    const [ax,ay]=line[n-1], [bx,by]=line[n];
    const t=Math.max(0,Math.min(1,((x-ax)*(bx-ax)+(y-ay)*(by-ay))/((bx-ax)**2+(by-ay)**2)));
    if(Math.hypot(x-ax-t*(bx-ax),y-ay-t*(by-ay))<1.2) stroke=true;
  }
  pixels.set(stroke?[255,255,255,255]:[181,99,37,inside?255:0],i);
}
const mask=Buffer.alloc(size*4);
const dib=Buffer.alloc(40);
dib.writeUInt32LE(40,0);dib.writeInt32LE(size,4);dib.writeInt32LE(size*2,8);dib.writeUInt16LE(1,12);dib.writeUInt16LE(32,14);dib.writeUInt32LE(pixels.length+mask.length,20);
const header=Buffer.alloc(22);
header.writeUInt16LE(1,2);header.writeUInt16LE(1,4);header[6]=size;header[7]=size;header.writeUInt16LE(1,10);header.writeUInt16LE(32,12);header.writeUInt32LE(dib.length+pixels.length+mask.length,14);header.writeUInt32LE(22,18);
mkdirSync('src-tauri/icons',{recursive:true});
writeFileSync('src-tauri/icons/icon.ico',Buffer.concat([header,dib,pixels,mask]));
