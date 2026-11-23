use crate::{firewall, storage::AppUsage};
use ferrisetw::{
    parser::Parser,
    provider::{
        kernel_providers::{self, KernelProvider},
        Provider,
    },
    schema_locator::SchemaLocator,
    trace::KernelTrace,
    EventRecord,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};

#[derive(Serialize, Deserialize)]
struct Frame {
    apps: Vec<AppUsage>,
    error: Option<String>,
}
fn process_path(pid: u32, event_time: i64) -> String {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let times = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
        let born = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
        let mut buffer = [0u16; 32768];
        let mut length = buffer.len() as u32;
        let result = if times.is_ok() && event_time > 0 && born <= event_time as u64 {
            QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut length,
            )
            .ok()
            .map(|_| String::from_utf16_lossy(&buffer[..length as usize]))
            .unwrap_or_default()
        } else {
            String::new()
        };
        let _ = CloseHandle(handle);
        result
    }
}
fn callback(
    data: crate::monitoring::ByteMap,
    failures: Arc<AtomicBool>,
) -> impl FnMut(&EventRecord, &SchemaLocator) + Send + 'static {
    move |record, locator| {
        let opcode = record.opcode();
        if !matches!(opcode, 10 | 11 | 26 | 27) {
            return;
        }
        let Ok(schema) = locator.event_schema(record) else {
            failures.store(true, Ordering::Relaxed);
            return;
        };
        let parser = Parser::create(record, &schema);
        let Ok(size) = parser.try_parse::<u32>("size") else {
            failures.store(true, Ordering::Relaxed);
            return;
        };
        let path = parser
            .try_parse::<u32>("PID")
            .map(|pid| process_path(pid, record.raw_timestamp()))
            .unwrap_or_default();
        if let Ok(mut map) = data.lock() {
            let path = if map.len() >= 1024 && !map.contains_key(&path) {
                String::new()
            } else {
                path
            };
            let entry = map.entry(path).or_default();
            if opcode == 11 || opcode == 27 {
                entry.0 = entry.0.saturating_add(size as u64);
            } else {
                entry.1 = entry.1.saturating_add(size as u64);
            }
        }
    }
}
pub fn helper(port: u16, token: &str) -> Result<(), String> {
    if token.len() != 32 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid session token".into());
    }
    let mut stream =
        TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    writeln!(stream, "{token}").map_err(|e| e.to_string())?;
    let data = Arc::new(Mutex::new(HashMap::new()));
    let failures = Arc::new(AtomicBool::new(false));
    let tcp = Provider::kernel(&kernel_providers::TCP_IP_PROVIDER)
        .add_callback(callback(data.clone(), failures.clone()))
        .build();
    let mut udp = KernelProvider {
        guid: Default::default(),
        flags: 0x10000,
    };
    udp.guid.data1 = 0xbf3a50c5;
    udp.guid.data2 = 0xa9c9;
    udp.guid.data3 = 0x4988;
    udp.guid.data4 = [0xa0, 0x05, 0x2d, 0xf0, 0xb7, 0xc8, 0x0f, 0x80];
    let udp = Provider::kernel(&udp)
        .add_callback(callback(data.clone(), failures.clone()))
        .build();
    let trace = match KernelTrace::new()
        .named(format!("NetControl-{}", uuid::Uuid::new_v4().simple()))
        .enable(tcp)
        .enable(udp)
        .start_and_process()
    {
        Ok(trace) => trace,
        Err(e) => {
            let _ = writeln!(
                stream,
                "{}",
                serde_json::to_string(&Frame {
                    apps: vec![],
                    error: Some(format!("Could not start Windows event tracing: {e:?}"))
                })
                .unwrap()
            );
            return Err(format!("{e:?}"));
        }
    };
    loop {
        let mut ping = [0u8; 1];
        if stream.read_exact(&mut ping).is_err() {
            break;
        }
        let mut map = data.lock().map_err(|e| e.to_string())?;
        let apps = std::mem::take(&mut *map)
            .into_iter()
            .map(|(path, (downloaded, uploaded))| AppUsage {
                path,
                downloaded,
                uploaded,
            })
            .collect();
        drop(map);
        let frame = Frame {
            apps,
            error: if failures.swap(false, Ordering::Relaxed) {
                Some("Some Windows network events could not be decoded; application totals are incomplete".into())
            } else {
                None
            },
        };
        if writeln!(
            stream,
            "{}",
            serde_json::to_string(&frame).map_err(|e| e.to_string())?
        )
        .is_err()
        {
            break;
        }
    }
    trace.stop().map_err(|e| format!("{e:?}"))?;
    Ok(())
}
pub fn enable(state: Arc<crate::AppState>) -> Result<(), String> {
    let mut m = state.monitor.lock().map_err(|e| e.to_string())?;
    if m.helper_active.load(Ordering::Relaxed) || m.snapshot.attribution == "Connecting" {
        return Ok(());
    }
    m.helper_stop.store(false, Ordering::Relaxed);
    m.snapshot.attribution = "Connecting".into();
    let stop = m.helper_stop.clone();
    let active = m.helper_active.clone();
    let incoming = m.incoming.clone();
    drop(m);
    let listener =
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let token = uuid::Uuid::new_v4().simple().to_string();
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    if let Err(e) = firewall::elevate(&format!("--monitor-helper {port} {token}"), false) {
        if let Ok(mut m) = state.monitor.lock() {
            m.snapshot.attribution = "Disabled".into();
        }
        return Err(e);
    }
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut reader = loop {
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .map_err(|e| e.to_string())?;
                        stream
                            .set_write_timeout(Some(Duration::from_secs(3)))
                            .map_err(|e| e.to_string())?;
                        let mut reader = BufReader::new(stream);
                        let mut hello = String::new();
                        if reader.by_ref().take(128).read_line(&mut hello).is_ok()
                            && hello.trim() == token
                        {
                            break reader;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(100))
                    }
                    Err(e) => return Err(e.to_string()),
                }
                if Instant::now() > deadline {
                    return Err("The application monitoring helper did not connect".into());
                }
            };
            active.store(true, Ordering::Relaxed);
            while !stop.load(Ordering::Relaxed) {
                reader
                    .get_mut()
                    .write_all(&[1])
                    .map_err(|e| e.to_string())?;
                let mut line = String::new();
                reader
                    .by_ref()
                    .take(1048576)
                    .read_line(&mut line)
                    .map_err(|e| e.to_string())?;
                let frame: Frame = serde_json::from_str(&line).map_err(|e| e.to_string())?;
                if let Some(error) = frame.error {
                    if let Ok(mut m) = state.monitor.lock() {
                        m.snapshot.error = error;
                    }
                }
                let mut map = incoming.lock().map_err(|e| e.to_string())?;
                for app in frame.apps {
                    let entry = map.entry(app.path).or_default();
                    entry.0 = entry.0.saturating_add(app.downloaded);
                    entry.1 = entry.1.saturating_add(app.uploaded);
                }
                drop(map);
                std::thread::sleep(Duration::from_secs(1));
            }
            Ok(())
        })();
        active.store(false, Ordering::Relaxed);
        if let Ok(mut m) = state.monitor.lock() {
            m.snapshot.attribution = match result {
                Ok(()) => "Disabled".into(),
                Err(e) => format!("Unavailable: {e}"),
            };
        }
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires an administrator account and Windows kernel event tracing"]
    fn attributes_real_tcp_and_udp_transfers() {
        let data = Arc::new(Mutex::new(HashMap::new()));
        let udp_data = Arc::new(Mutex::new(HashMap::new()));
        let failures = Arc::new(AtomicBool::new(false));
        let tcp = Provider::kernel(&kernel_providers::TCP_IP_PROVIDER)
            .add_callback(callback(data.clone(), failures.clone()))
            .build();
        let mut udp = KernelProvider {
            guid: Default::default(),
            flags: 0x10000,
        };
        udp.guid.data1 = 0xbf3a50c5;
        udp.guid.data2 = 0xa9c9;
        udp.guid.data3 = 0x4988;
        udp.guid.data4 = [0xa0, 0x05, 0x2d, 0xf0, 0xb7, 0xc8, 0x0f, 0x80];
        let udp = Provider::kernel(&udp)
            .add_callback(callback(udp_data.clone(), failures.clone()))
            .build();
        let trace = KernelTrace::new()
            .named(format!("NetControl-test-{}", uuid::Uuid::new_v4().simple()))
            .enable(tcp)
            .enable(udp)
            .start_and_process()
            .expect("Start ETW");
        std::thread::sleep(Duration::from_secs(1));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let receiver = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            socket.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes.len(), 1048576);
        });
        let mut sender = TcpStream::connect(address).unwrap();
        sender.write_all(&vec![42u8; 1048576]).unwrap();
        drop(sender);
        receiver.join().unwrap();
        let server = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let destination = server.local_addr().unwrap();
        let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        for _ in 0..20 {
            client.send_to(&[7u8; 1024], destination).unwrap();
            let mut buf = [0u8; 2048];
            assert_eq!(server.recv(&mut buf).unwrap(), 1024);
        }
        std::thread::sleep(Duration::from_secs(3));
        trace.stop().unwrap();
        assert!(
            !failures.load(Ordering::Relaxed),
            "Windows event parsing failed"
        );
        let map = data.lock().unwrap();
        let own = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches("\\\\?\\")
            .to_string();
        let own_bytes = map
            .iter()
            .filter(|(path, _)| path.eq_ignore_ascii_case(&own))
            .map(|(_, v)| v.0 + v.1)
            .sum::<u64>();
        assert!(
            own_bytes >= 1048576,
            "Expected attributed transfer bytes; got {own_bytes}, observed paths: {:?}",
            map.keys().collect::<Vec<_>>()
        );
        let udp_map = udp_data.lock().unwrap();
        let udp_bytes = udp_map.iter().filter(|(path, _)| path.eq_ignore_ascii_case(&own)).map(|(_, v)| v.0 + v.1).sum::<u64>();
        assert!(udp_bytes >= 20480, "Expected attributed UDP bytes; got {udp_bytes}");
    }
}
