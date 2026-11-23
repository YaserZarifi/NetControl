use std::collections::HashMap;
use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
pub struct AdapterCounters {
    pub downloaded: u64,
    pub uploaded: u64,
    pub online: bool,
    pub physical: bool,
    pub description: String,
}
impl AdapterCounters {
    pub fn total_received(&self) -> u64 {
        self.downloaded
    }
    pub fn total_transmitted(&self) -> u64 {
        self.uploaded
    }
}
fn wide(value: &[u16]) -> String {
    String::from_utf16_lossy(&value[..value.iter().position(|v| *v == 0).unwrap_or(value.len())])
}
pub fn read() -> Result<HashMap<String, AdapterCounters>, String> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        let result = GetIfTable2(&mut table);
        if result.0 != 0 {
            return Err(format!("Windows adapter query failed: {}", result.0));
        }
        if table.is_null() {
            return Err("Windows returned no interface table".into());
        }
        let rows =
            std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let mut adapters = HashMap::new();
        for row in rows {
            if row.Type == 24 {
                continue;
            }
            adapters.insert(
                wide(&row.Alias),
                AdapterCounters {
                    downloaded: row.InOctets,
                    uploaded: row.OutOctets,
                    online: row.OperStatus.0 == 1,
                    physical: row.InterfaceAndOperStatusFlags._bitfield & 1 != 0,
                    description: wide(&row.Description),
                },
            );
        }
        FreeMibTable(table.cast());
        Ok(adapters)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn queries_windows_interfaces() {
        let result = super::read().unwrap();
        assert!(!result.is_empty());
    }
}
