use std::time::Instant;

use uuid::Uuid;

use crate::protocol::MetricsSnapshot;

const MAX_BUFFER_SIZE: usize = 64 * 1024;
const FRAME_PREFIX: &str = "WG_METRICS_V1 ";

pub const METRICS_COLLECTOR_COMMAND: &str = r#"sh -c '
LC_ALL=C
cpu_count=0
while read cpu rest; do
    case "$cpu" in
        cpu[0-9]*) cpu_count=$((cpu_count + 1)) ;;
    esac
done < /proc/stat
[ "$cpu_count" -gt 0 ] || cpu_count=1
while :; do
    read _ user nice system idle iowait irq softirq steal _ _ < /proc/stat || exit 0
    cpu_idle=$((${idle:-0} + ${iowait:-0}))
    cpu_total=$((${user:-0} + ${nice:-0} + ${system:-0} + ${idle:-0} + ${iowait:-0} + ${irq:-0} + ${softirq:-0} + ${steal:-0}))

    mem_total=0
    mem_available=0
    mem_free=0
    buffers=0
    cached=0
    while read key value rest; do
        case "$key" in
            MemTotal:) mem_total=${value:-0} ;;
            MemAvailable:) mem_available=${value:-0} ;;
            MemFree:) mem_free=${value:-0} ;;
            Buffers:) buffers=${value:-0} ;;
            Cached:) cached=${value:-0} ;;
        esac
    done < /proc/meminfo
    [ "$mem_available" -gt 0 ] || mem_available=$((mem_free + buffers + cached))

    read load1 _ < /proc/loadavg || load1=0
    read uptime _ < /proc/uptime || uptime=0

    default_iface=""
    if [ -r /proc/net/route ]; then
        while read iface destination rest; do
            if [ "$destination" = "00000000" ]; then
                default_iface=$iface
                break
            fi
        done < /proc/net/route
    fi

    rx=0
    tx=0
    net_iface="$default_iface"
    while IFS= read -r line; do
        case "$line" in
            *:*)
                iface=${line%%:*}
                set -- $iface
                iface=${1:-}
                [ "$iface" = "lo" ] && continue
                [ -n "$default_iface" ] && [ "$iface" != "$default_iface" ] && continue
                rest=${line#*:}
                set -- $rest
                rx=$((rx + ${1:-0}))
                tx=$((tx + ${9:-0}))
                ;;
        esac
    done < /proc/net/dev
    [ -n "$net_iface" ] || net_iface="*"

    set -- $(df -Pk / 2>/dev/null | tail -n 1)
    disk_percent=${5:-0}
    disk_percent=${disk_percent%\%}

    printf "WG_METRICS_V1 cpu_total=%s cpu_idle=%s mem_total_kb=%s mem_available_kb=%s rx_bytes=%s tx_bytes=%s load1=%s disk_percent=%s uptime_seconds=%s cpu_count=%s net_iface=%s\n" \
        "$cpu_total" "$cpu_idle" "$mem_total" "$mem_available" "$rx" "$tx" "$load1" "$disk_percent" "$uptime" "$cpu_count" "$net_iface"
    sleep 1
done
'"#;

#[derive(Debug)]
struct RawMetrics {
    cpu_total: u64,
    cpu_idle: u64,
    mem_total_kb: u64,
    mem_available_kb: u64,
    rx_bytes: u64,
    tx_bytes: u64,
    load1: f64,
    disk_percent: f64,
    uptime_seconds: f64,
    cpu_count: u32,
    net_iface: Option<String>,
}

#[derive(Debug)]
struct PreviousMetrics {
    cpu_total: u64,
    cpu_idle: u64,
    rx_bytes: u64,
    tx_bytes: u64,
    uptime_seconds: f64,
    received_at: Instant,
}

#[derive(Debug, Default)]
pub struct MetricsDecodeResult {
    pub snapshots: Vec<MetricsSnapshot>,
    pub became_available: bool,
}

#[derive(Debug, Default)]
pub struct MetricsCollectorState {
    channel_id: Option<Uuid>,
    buffer: Vec<u8>,
    previous: Option<PreviousMetrics>,
    available: bool,
    stopping: bool,
}

impl MetricsCollectorState {
    pub fn start(&mut self, channel_id: Uuid) -> bool {
        if self.channel_id.is_some() {
            return false;
        }
        self.channel_id = Some(channel_id);
        self.buffer.clear();
        self.previous = None;
        self.available = false;
        self.stopping = false;
        true
    }

    pub fn is_channel(&self, channel_id: Uuid) -> bool {
        self.channel_id == Some(channel_id)
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn request_stop(&mut self) -> Option<Uuid> {
        self.stopping = true;
        self.channel_id
    }

    pub fn finish(&mut self, channel_id: Uuid) -> Option<bool> {
        if !self.is_channel(channel_id) {
            return None;
        }
        let was_stopping = self.stopping;
        *self = Self::default();
        Some(was_stopping)
    }

    pub fn feed(&mut self, channel_id: Uuid, data: &[u8]) -> MetricsDecodeResult {
        let mut result = MetricsDecodeResult::default();
        if self.stopping || !self.is_channel(channel_id) {
            return result;
        }

        self.buffer.extend_from_slice(data);
        if self.buffer.len() > MAX_BUFFER_SIZE {
            self.buffer.clear();
            return result;
        }

        while let Some(newline) = self.buffer.iter().position(|b| *b == b'\n') {
            let line = self.buffer.drain(..=newline).collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&line);
            let Some(raw) = parse_frame(line.trim()) else {
                continue;
            };

            let now = Instant::now();
            let snapshot = build_snapshot(&raw, self.previous.as_ref(), now);
            self.previous = Some(PreviousMetrics {
                cpu_total: raw.cpu_total,
                cpu_idle: raw.cpu_idle,
                rx_bytes: raw.rx_bytes,
                tx_bytes: raw.tx_bytes,
                uptime_seconds: raw.uptime_seconds,
                received_at: now,
            });

            if !self.available {
                self.available = true;
                result.became_available = true;
            }
            result.snapshots.push(snapshot);
        }
        result
    }
}

fn parse_frame(line: &str) -> Option<RawMetrics> {
    let fields = line.strip_prefix(FRAME_PREFIX)?;
    let mut cpu_total = None;
    let mut cpu_idle = None;
    let mut mem_total_kb = None;
    let mut mem_available_kb = None;
    let mut rx_bytes = None;
    let mut tx_bytes = None;
    let mut load1 = None;
    let mut disk_percent = None;
    let mut uptime_seconds = None;
    let mut cpu_count = None;
    let mut net_iface = None;

    for field in fields.split_whitespace() {
        let (key, value) = field.split_once('=')?;
        match key {
            "cpu_total" => cpu_total = value.parse().ok(),
            "cpu_idle" => cpu_idle = value.parse().ok(),
            "mem_total_kb" => mem_total_kb = value.parse().ok(),
            "mem_available_kb" => mem_available_kb = value.parse().ok(),
            "rx_bytes" => rx_bytes = value.parse().ok(),
            "tx_bytes" => tx_bytes = value.parse().ok(),
            "load1" => load1 = value.parse().ok(),
            "disk_percent" => disk_percent = value.parse().ok(),
            "uptime_seconds" => uptime_seconds = value.parse().ok(),
            "cpu_count" => cpu_count = value.parse().ok(),
            "net_iface" => net_iface = (value != "*").then(|| value.to_owned()),
            _ => {}
        }
    }

    Some(RawMetrics {
        cpu_total: cpu_total?,
        cpu_idle: cpu_idle?,
        mem_total_kb: mem_total_kb?,
        mem_available_kb: mem_available_kb?,
        rx_bytes: rx_bytes?,
        tx_bytes: tx_bytes?,
        load1: load1?,
        disk_percent: disk_percent?,
        uptime_seconds: uptime_seconds?,
        cpu_count: cpu_count?,
        net_iface,
    })
}

fn build_snapshot(
    raw: &RawMetrics,
    previous: Option<&PreviousMetrics>,
    now: Instant,
) -> MetricsSnapshot {
    let cpu_percent = previous.and_then(|p| {
        let total_delta = raw.cpu_total.checked_sub(p.cpu_total)?;
        let idle_delta = raw.cpu_idle.checked_sub(p.cpu_idle)?;
        if total_delta == 0 || idle_delta > total_delta {
            return None;
        }
        Some(100.0 * (total_delta - idle_delta) as f64 / total_delta as f64)
    });

    let elapsed = previous.map(|p| {
        let remote_elapsed = raw.uptime_seconds - p.uptime_seconds;
        if remote_elapsed > 0.0 {
            remote_elapsed
        } else {
            now.duration_since(p.received_at).as_secs_f64()
        }
    });
    let rx_bytes_per_sec = previous.and_then(|p| {
        let elapsed = elapsed?;
        if elapsed <= 0.0 {
            return None;
        }
        let delta = raw.rx_bytes.checked_sub(p.rx_bytes)?;
        Some(delta as f64 / elapsed)
    });
    let tx_bytes_per_sec = previous.and_then(|p| {
        let elapsed = elapsed?;
        if elapsed <= 0.0 {
            return None;
        }
        let delta = raw.tx_bytes.checked_sub(p.tx_bytes)?;
        Some(delta as f64 / elapsed)
    });

    let memory_total_bytes = raw.mem_total_kb.saturating_mul(1024);
    let memory_used_bytes = raw
        .mem_total_kb
        .saturating_sub(raw.mem_available_kb)
        .saturating_mul(1024);

    MetricsSnapshot {
        cpu_percent,
        cpu_count: raw.cpu_count,
        memory_used_bytes,
        memory_total_bytes,
        load1: raw.load1,
        rx_bytes_per_sec,
        tx_bytes_per_sec,
        network_interface: raw.net_iface.clone(),
        disk_percent: raw.disk_percent.clamp(0.0, 100.0),
        uptime_seconds: raw.uptime_seconds,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const FRAME1: &str = "WG_METRICS_V1 cpu_total=1000 cpu_idle=700 mem_total_kb=8000000 mem_available_kb=5000000 rx_bytes=100000 tx_bytes=50000 load1=0.42 disk_percent=41 uptime_seconds=1234.5 cpu_count=4 net_iface=eth0";
    const FRAME2: &str = "WG_METRICS_V1 cpu_total=1200 cpu_idle=800 mem_total_kb=8000000 mem_available_kb=4900000 rx_bytes=104000 tx_bytes=51000 load1=0.50 disk_percent=42 uptime_seconds=1236.5 cpu_count=4 net_iface=eth0";

    #[test]
    fn parses_metrics_frame() {
        let raw = parse_frame(FRAME1).expect("valid frame");
        assert_eq!(raw.cpu_total, 1000);
        assert_eq!(raw.cpu_idle, 700);
        assert_eq!(raw.net_iface.as_deref(), Some("eth0"));
        assert_eq!(raw.cpu_count, 4);
    }

    #[test]
    fn computes_rates_from_counter_deltas() {
        let first = parse_frame(FRAME1).unwrap();
        let second = parse_frame(FRAME2).unwrap();
        let now = Instant::now();
        let previous = PreviousMetrics {
            cpu_total: first.cpu_total,
            cpu_idle: first.cpu_idle,
            rx_bytes: first.rx_bytes,
            tx_bytes: first.tx_bytes,
            uptime_seconds: first.uptime_seconds,
            received_at: now - Duration::from_secs(2),
        };
        let snapshot = build_snapshot(&second, Some(&previous), now);
        assert_eq!(snapshot.cpu_percent, Some(50.0));
        assert_eq!(snapshot.rx_bytes_per_sec, Some(2000.0));
        assert_eq!(snapshot.tx_bytes_per_sec, Some(500.0));
        assert_eq!(snapshot.memory_total_bytes, 8_192_000_000);
        assert_eq!(snapshot.memory_used_bytes, 3_174_400_000);
    }

    #[test]
    fn ignores_late_output_after_stop() {
        let channel = Uuid::new_v4();
        let mut state = MetricsCollectorState::default();
        assert!(state.start(channel));
        assert_eq!(state.request_stop(), Some(channel));
        let data = format!("{FRAME1}\n");
        let decoded = state.feed(channel, data.as_bytes());
        assert!(decoded.snapshots.is_empty());
        assert!(!decoded.became_available);
    }

    #[test]
    fn reassembles_split_frames() {
        let channel = Uuid::new_v4();
        let mut state = MetricsCollectorState::default();
        assert!(state.start(channel));
        let midpoint = FRAME1.len() / 2;
        let first = state.feed(channel, &FRAME1.as_bytes()[..midpoint]);
        assert!(first.snapshots.is_empty());
        let second_half = format!("{}\n", &FRAME1[midpoint..]);
        let second = state.feed(channel, second_half.as_bytes());
        assert!(second.became_available);
        assert_eq!(second.snapshots.len(), 1);
        assert_eq!(second.snapshots[0].cpu_count, 4);
    }
}
