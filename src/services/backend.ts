import { invoke, isTauri } from '@tauri-apps/api/core';
export type Settings = { theme: 'system' | 'light' | 'dark'; units: 'decimal' | 'binary'; retentionDays: number; adapter: string; minimizeToTray: boolean; startMinimized: boolean; launchAtStartup: boolean };
export type Foundation = { version: string; databaseReady: boolean; settings: Settings };
export type AppUsage = { path: string; downloaded: number; uploaded: number };
export type DailyUsage = { date: string; downloaded: number; uploaded: number; coverageMs: number };
export type Point = { time: number; download: number; upload: number };
export type Snapshot = { paused: boolean; adapter: string; adapters: {name:string;virtualAdapter:boolean}[]; points: Point[]; download:number;upload:number;status:string;attribution:string;applications:AppUsage[];error:string };
export type History = { days: DailyUsage[]; applications: AppUsage[]; blockedPaths: string[] };
export const desktop = isTauri();
const defaults: Settings = { theme: 'system', units: 'decimal', retentionDays: 90, adapter:'',minimizeToTray:true,startMinimized:false,launchAtStartup:false };
function previewSettings(): Settings {
  try {
    const s = JSON.parse(localStorage.getItem('netcontrol-preview') ?? 'null');
    if (s && ['system', 'light', 'dark'].includes(s.theme) && ['decimal', 'binary'].includes(s.units) && [30,90,365].includes(s.retentionDays)) return {...defaults,...s};
  } catch { /* Use defaults when browser storage is unavailable. */ }
  return defaults;
}
export async function loadFoundation(): Promise<Foundation> { return desktop ? invoke('get_foundation') : {version:'0.1.0',databaseReady:false,settings:previewSettings()}; }
export async function saveSettings(settings: Settings): Promise<void> {
  if (desktop) await invoke('save_settings',{settings}); else localStorage.setItem('netcontrol-preview',JSON.stringify(settings));
}
export async function snapshot(): Promise<Snapshot> {return desktop ? invoke('get_snapshot') : {paused:false,adapter:'',adapters:[],points:[],download:0,upload:0,status:'Browser preview',attribution:'Desktop required',applications:[],error:''};}
export async function history(from:string,to:string): Promise<History> {return desktop ? invoke('get_history',{from,to}) : {days:[],applications:[],blockedPaths:[]};}
export async function command(name:string,args?:Record<string,unknown>):Promise<void>{if(!desktop)throw new Error('Open the installed Windows app to use this feature.');await invoke(name,args);}
export function localDate(date=new Date()):string {return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;}
export function formatBytes(value:number,binary=false):string {const base=binary?1024:1000;const labels=binary?['B','KiB','MiB','GiB','TiB']:['B','KB','MB','GB','TB'];let i=0;while(value>=base&&i<labels.length-1){value/=base;i++;}return `${value.toFixed(i?1:0)} ${labels[i]}`;}
export function exportCsv(days:DailyUsage[],apps:AppUsage[],from:string,to:string){
  const quote=(s:string|number)=>`"${String(s).replaceAll('"','""')}"`;
  const rows=[['Date','Downloaded bytes','Uploaded bytes','Measured milliseconds'],...days.map(d=>[d.date,d.downloaded,d.uploaded,d.coverageMs]),[],['Application path (TCP/UDP across all adapters)','Downloaded bytes','Uploaded bytes'],...apps.map(a=>[a.path||'System / Unattributed',a.downloaded,a.uploaded])];
  const blob=new Blob(['\uFEFF'+rows.map(row=>row.map(quote).join(',')).join('\r\n')],{type:'text/csv;charset=utf-8'});
  const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download=`NetControl-${from}-to-${to}.csv`;link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}

export async function appHistory(path:string,from:string,to:string):Promise<DailyUsage[]>{return desktop?invoke('get_app_history',{path,from,to}):[];}
