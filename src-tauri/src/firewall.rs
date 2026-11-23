use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use windows::{core::{w, BSTR, PCWSTR}, Win32::{Foundation::{CloseHandle, WAIT_OBJECT_0}, NetworkManagement::WindowsFirewall::*, System::{Com::{CoCreateInstance,CoInitializeEx,CoUninitialize,CLSCTX_INPROC_SERVER,COINIT_APARTMENTTHREADED},Threading::{GetExitCodeProcess,WaitForSingleObject}}, UI::{Shell::{ShellExecuteExW,SHELLEXECUTEINFOW,SEE_MASK_NOCLOSEPROCESS},WindowsAndMessaging::SW_HIDE}}};

pub struct Com;
impl Com { pub fn init()->Result<Self,String>{unsafe{CoInitializeEx(None,COINIT_APARTMENTTHREADED).ok().map_err(|e|e.to_string())?;}Ok(Self)} }
impl Drop for Com {fn drop(&mut self){unsafe{CoUninitialize();}}}
const GROUP: &str="NetControl application restrictions v1";
pub fn rule_id(path:&str)->String {format!("NetControl.Outbound.v1.{:x}",Sha256::digest(path.to_lowercase().as_bytes()))}
pub fn validate_path(path:&str,blocking:bool)->Result<String,String>{
    if path.len()>32700 || path.contains(['\0','\r','\n','"']) {return Err("Invalid executable path".into());}
    let original=Path::new(path);
    if !original.is_absolute() || !path.get(1..3).is_some_and(|s|s==":\\") || !path.to_lowercase().ends_with(".exe") {return Err("Choose a full local Windows .exe path".into());}
    let resolved=if original.exists(){std::fs::canonicalize(original).map_err(|e|e.to_string())?.to_string_lossy().trim_start_matches("\\\\?\\").to_string()}else if blocking{return Err("The executable does not exist".into())}else{path.into()};
    if !resolved.get(1..3).is_some_and(|s|s==":\\") {return Err("Network and device paths are not supported".into());}
    if blocking {
        if !Path::new(&resolved).is_file(){return Err("Choose an executable file".into());}
        let mut buffer=[0u16;32768];
        let n=unsafe{windows::Win32::System::SystemInformation::GetWindowsDirectoryW(Some(&mut buffer))} as usize;
        if n==0 || n>=buffer.len(){return Err("Could not verify the Windows system directory".into());}
        let windows=String::from_utf16_lossy(&buffer[..n]).to_lowercase()+"\\";
        if resolved.to_lowercase().starts_with(&windows){return Err("Windows system executables are protected".into());}
        let own=std::env::current_exe().map_err(|e|e.to_string())?.to_string_lossy().trim_start_matches("\\\\?\\").to_string();
        if resolved.eq_ignore_ascii_case(&own){return Err("NetControl cannot block itself".into());}
    }
    Ok(resolved)
}
fn owns(rule:&INetFwRule,path:&str)->bool{unsafe{rule.Grouping().is_ok_and(|s|s.to_string()==GROUP)&&rule.ApplicationName().is_ok_and(|s|s.to_string().eq_ignore_ascii_case(path))}}
pub fn blocked_paths(paths:&[String])->Result<Vec<String>,String>{
    let _com=Com::init()?;
    unsafe{
        let policy:INetFwPolicy2=CoCreateInstance(&NetFwPolicy2,None,CLSCTX_INPROC_SERVER).map_err(|e|e.to_string())?;
        let rules=policy.Rules().map_err(|e|e.to_string())?;
        Ok(paths.iter().filter(|path|rules.Item(&BSTR::from(rule_id(path).as_str())).is_ok_and(|r|owns(&r,path)&&r.Enabled().is_ok_and(|v|v.0!=0)&&r.Action().is_ok_and(|v|v==NET_FW_ACTION_BLOCK)&&r.Direction().is_ok_and(|v|v==NET_FW_RULE_DIR_OUT))).cloned().collect())
    }
}
pub fn apply(path:&str,blocking:bool)->Result<(),String>{
    let path=validate_path(path,blocking)?;
    let _com=Com::init()?;
    unsafe{
        let policy:INetFwPolicy2=CoCreateInstance(&NetFwPolicy2,None,CLSCTX_INPROC_SERVER).map_err(|e|e.to_string())?;
        let rules=policy.Rules().map_err(|e|e.to_string())?;
        let id=BSTR::from(rule_id(&path).as_str());
        if let Ok(existing)=rules.Item(&id){
            if !owns(&existing,&path){return Err("A different rule uses this identifier; it was preserved".into());}
            if !blocking {return rules.Remove(&id).map_err(|e|e.to_string());}
        }else if !blocking{return Ok(())}
        let rule:INetFwRule=CoCreateInstance(&NetFwRule,None,CLSCTX_INPROC_SERVER).map_err(|e|e.to_string())?;
        rule.SetName(&id).map_err(|e|e.to_string())?;
        rule.SetApplicationName(&BSTR::from(path.as_str())).map_err(|e|e.to_string())?;
        rule.SetGrouping(&BSTR::from(GROUP)).map_err(|e|e.to_string())?;
        rule.SetDescription(&BSTR::from("Outbound restriction created by NetControl. Remove it through NetControl or Windows Firewall.")).map_err(|e|e.to_string())?;
        rule.SetProtocol(256).map_err(|e|e.to_string())?;
        rule.SetDirection(NET_FW_RULE_DIR_OUT).map_err(|e|e.to_string())?;
        rule.SetProfiles(NET_FW_PROFILE2_ALL.0).map_err(|e|e.to_string())?;
        rule.SetAction(NET_FW_ACTION_BLOCK).map_err(|e|e.to_string())?;
        rule.SetEnabled(true.into()).map_err(|e|e.to_string())?;
        rules.Add(&rule).map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn elevate(arguments:&str,wait:bool)->Result<(),String>{
    // Arguments are fixed switches, numeric ports, and hex-encoded values only.
    if !arguments.bytes().all(|b|b.is_ascii_alphanumeric()||b==b' '||b==b'-'){return Err("Invalid helper arguments".into());}
    let _com=Com::init()?;
    let exe:Vec<u16>=std::env::current_exe().map_err(|e|e.to_string())?.as_os_str().encode_wide().chain(Some(0)).collect();
    let args:Vec<u16>=arguments.encode_utf16().chain(Some(0)).collect();
    unsafe{
        let mut info=SHELLEXECUTEINFOW{cbSize:std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,fMask:SEE_MASK_NOCLOSEPROCESS,lpVerb:w!("runas"),lpFile:PCWSTR(exe.as_ptr()),lpParameters:PCWSTR(args.as_ptr()),nShow:SW_HIDE.0,..Default::default()};
        ShellExecuteExW(&mut info).map_err(|e|format!("Administrator request cancelled or unavailable: {e}"))?;
        if wait {
            let result=WaitForSingleObject(info.hProcess,Duration::from_secs(120).as_millis() as u32);
            let mut code=1;
            let read=GetExitCodeProcess(info.hProcess,&mut code);
            let _=CloseHandle(info.hProcess);
            if result!=WAIT_OBJECT_0{return Err("The administrator operation did not finish in time. Check the rule before retrying.".into());}
            read.map_err(|e|e.to_string())?;
            if code!=0{return Err("Windows rejected the firewall change. Verify administrator rights and Windows Firewall availability.".into());}
        }else{let _=CloseHandle(info.hProcess);}
    }
    Ok(())
}
use std::os::windows::ffi::OsStrExt;
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn rejects_shell_and_network_paths(){for p in ["relative.exe","\\\\server\\x.exe","C:\\app.exe\n","C:\\a\".exe"]{assert!(validate_path(p,true).is_err());}}
    #[test]fn rule_identity_is_case_insensitive(){assert_eq!(rule_id("C:\\App.exe"),rule_id("c:\\app.EXE"));assert_ne!(rule_id("C:\\App.exe"),rule_id("C:\\Other.exe"));}
}
#[cfg(test)]
mod native_tests {
    use super::*;
    struct Cleanup(String);
    impl Drop for Cleanup{fn drop(&mut self){let _=apply(&self.0,false);let _=std::fs::remove_file(&self.0);}}
    #[test]
    #[ignore = "Requires administrator rights to create and remove a disposable firewall rule"]
    fn creates_and_removes_only_its_disposable_rule(){
        let path=std::env::temp_dir().join(format!("netcontrol-firewall-test-{}.exe",uuid::Uuid::new_v4()));
        std::fs::write(&path,[]).unwrap();
        let path=path.to_string_lossy().to_string();let _cleanup=Cleanup(path.clone());
        assert!(blocked_paths(std::slice::from_ref(&path)).unwrap().is_empty());
        apply(&path,true).unwrap();
        assert_eq!(blocked_paths(std::slice::from_ref(&path)).unwrap(),vec![path.clone()]);
        apply(&path,false).unwrap();
        assert!(blocked_paths(std::slice::from_ref(&path)).unwrap().is_empty());
        apply(&path,false).unwrap();
    }
}
