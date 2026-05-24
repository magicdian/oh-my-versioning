use std::net::UdpSocket;
use std::time::Duration;

use crate::core::date::LogicalDate;
use crate::core::time::{LastTimeSource, TimeSource};
use crate::errors::{NtpError, OmvError};

const NTP_PACKET_SIZE: usize = 48;
const NTP_UNIX_EPOCH_OFFSET_SECONDS: u64 = 2_208_988_800;

#[derive(Debug, Clone)]
pub struct NtpTimeSource {
    server: String,
    timeout: Duration,
}

impl Default for NtpTimeSource {
    fn default() -> Self {
        Self {
            server: String::from("time.google.com:123"),
            timeout: Duration::from_secs(2),
        }
    }
}

impl NtpTimeSource {
    pub fn new(server: impl Into<String>, timeout: Duration) -> Self {
        Self {
            server: server.into(),
            timeout,
        }
    }

    fn query_unix_seconds(&self) -> Result<i64, OmvError> {
        let mut request = [0u8; NTP_PACKET_SIZE];
        request[0] = 0x1B;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|err| ntp_unavailable(format!("socket bind failed: {err}")))?;
        socket
            .set_read_timeout(Some(self.timeout))
            .map_err(|err| ntp_unavailable(format!("set read timeout failed: {err}")))?;
        socket
            .set_write_timeout(Some(self.timeout))
            .map_err(|err| ntp_unavailable(format!("set write timeout failed: {err}")))?;
        socket
            .connect(&self.server)
            .map_err(|err| ntp_unavailable(format!("connect to {} failed: {err}", self.server)))?;

        socket
            .send(&request)
            .map_err(|err| ntp_unavailable(format!("ntp request send failed: {err}")))?;

        let mut response = [0u8; NTP_PACKET_SIZE];
        let size = socket
            .recv(&mut response)
            .map_err(|err| ntp_unavailable(format!("ntp response recv failed: {err}")))?;

        extract_unix_seconds(&response[..size])
    }

    fn query_date(&self) -> Result<LogicalDate, OmvError> {
        let unix_seconds = self.query_unix_seconds()?;
        Ok(LogicalDate::from_unix_days(unix_seconds / 86_400))
    }
}

impl TimeSource for NtpTimeSource {
    fn source(&self) -> LastTimeSource {
        LastTimeSource::Ntp
    }

    fn today(&self) -> Result<LogicalDate, OmvError> {
        self.query_date()
    }

    fn unix_seconds(&self) -> Result<i64, OmvError> {
        self.query_unix_seconds()
    }
}

fn extract_unix_seconds(packet: &[u8]) -> Result<i64, OmvError> {
    if packet.len() < NTP_PACKET_SIZE {
        return Err(ntp_unavailable(format!(
            "ntp response too short: expected {NTP_PACKET_SIZE}, got {}",
            packet.len()
        )));
    }

    let transmit_seconds = u32::from_be_bytes([packet[40], packet[41], packet[42], packet[43]]);
    let transmit_seconds = transmit_seconds as u64;

    if transmit_seconds < NTP_UNIX_EPOCH_OFFSET_SECONDS {
        return Err(ntp_unavailable(
            "ntp transmit timestamp is before unix epoch".to_owned(),
        ));
    }

    Ok((transmit_seconds - NTP_UNIX_EPOCH_OFFSET_SECONDS) as i64)
}

#[allow(dead_code)]
fn extract_unix_days(packet: &[u8]) -> Result<i64, OmvError> {
    Ok(extract_unix_seconds(packet)? / 86_400)
}

fn ntp_unavailable(reason: String) -> OmvError {
    OmvError::Ntp(NtpError::Unavailable(reason))
}

#[cfg(test)]
mod tests {
    use super::{extract_unix_days, extract_unix_seconds};

    #[test]
    fn extract_unix_seconds_reads_transmit_timestamp() {
        let mut packet = [0u8; 48];
        // 1970-01-02 00:00:00 UTC in NTP seconds.
        let transmit = 2_208_988_800u32 + 86_400;
        packet[40..44].copy_from_slice(&transmit.to_be_bytes());

        let seconds = extract_unix_seconds(&packet).expect("packet should parse");
        assert_eq!(seconds, 86_400);
    }

    #[test]
    fn extract_unix_days_reads_transmit_timestamp() {
        let mut packet = [0u8; 48];
        // 1970-01-02 00:00:00 UTC in NTP seconds.
        let transmit = 2_208_988_800u32 + 86_400;
        packet[40..44].copy_from_slice(&transmit.to_be_bytes());

        let days = extract_unix_days(&packet).expect("packet should parse");
        assert_eq!(days, 1);
    }

    #[test]
    fn extract_unix_seconds_rejects_short_packets() {
        let packet = [0u8; 32];
        assert!(extract_unix_seconds(&packet).is_err());
    }

    #[test]
    fn extract_unix_days_rejects_short_packets() {
        let packet = [0u8; 32];
        assert!(extract_unix_days(&packet).is_err());
    }
}
