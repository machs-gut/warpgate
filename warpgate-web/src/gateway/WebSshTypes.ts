export type MetricsStatus =
    | 'starting'
    | 'available'
    | 'unavailable'
    | 'disabled'

export interface MetricsSnapshot {
    cpu_percent: number | null
    cpu_count: number
    memory_used_bytes: number
    memory_total_bytes: number
    load1: number
    rx_bytes_per_sec: number | null
    tx_bytes_per_sec: number | null
    network_interface: string | null
    disk_percent: number
    uptime_seconds: number
}

export interface MetricsViewState {
    status: MetricsStatus
    message: string | null
    snapshot: MetricsSnapshot | null
    history: MetricsSnapshot[]
    lastSampleAt: number | null
}
