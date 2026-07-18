import { invoke } from '@tauri-apps/api/core'

type RtcDiagnosticsSample = {
  peerConnections: number
  inboundVideoStreams: number
  outboundVideoStreams: number
  inboundBitrateBps: number
  outboundBitrateBps: number
  packetsLost: number
  framesDecoded: number
  framesDropped: number
  roundTripTimeMs?: number
  jitterMs?: number
}

type RtcReport = RTCStats & Record<string, unknown>

type RtcTotals = {
  inboundBytes: number
  outboundBytes: number
  inboundVideoStreams: number
  outboundVideoStreams: number
  packetsLost: number
  framesDecoded: number
  framesDropped: number
  roundTripTimeMs?: number
  jitterMs?: number
}

type PreviousSample = Pick<RtcTotals, 'inboundBytes' | 'outboundBytes'> & { timestamp: number }

const SAMPLE_INTERVAL_MS = 15_000
const trackedConnections = new Set<RTCPeerConnection>()

function numberValue(report: RtcReport, key: string): number {
  const value = report[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : 0
}

function optionalMilliseconds(seconds: number): number | undefined {
  return seconds > 0 ? Math.round(seconds * 1000) : undefined
}

function isVideoReport(report: RtcReport): boolean {
  return report.kind === 'video' || report.mediaType === 'video'
}

function track(connection: RTCPeerConnection) {
  trackedConnections.add(connection)
  const removeClosedConnection = () => {
    if (connection.connectionState === 'closed' || connection.iceConnectionState === 'closed') {
      trackedConnections.delete(connection)
    }
  }

  connection.addEventListener('connectionstatechange', removeClosedConnection)
  connection.addEventListener('iceconnectionstatechange', removeClosedConnection)
}

function installPeerConnectionTracker() {
  const nativePeerConnection = window.RTCPeerConnection
  if (!nativePeerConnection) return false

  const trackedPeerConnection = new Proxy(nativePeerConnection, {
    construct(target, args, newTarget) {
      const connection = Reflect.construct(target, args, newTarget) as RTCPeerConnection
      track(connection)
      return connection
    },
  })

  window.RTCPeerConnection = trackedPeerConnection as typeof RTCPeerConnection
  return true
}

async function collectTotals(): Promise<RtcTotals> {
  const totals: RtcTotals = {
    inboundBytes: 0,
    outboundBytes: 0,
    inboundVideoStreams: 0,
    outboundVideoStreams: 0,
    packetsLost: 0,
    framesDecoded: 0,
    framesDropped: 0,
  }

  for (const connection of trackedConnections) {
    const stats = await connection.getStats().catch(() => undefined)
    if (!stats) continue

    stats.forEach(report => {
      const rtcReport = report as RtcReport
      if (rtcReport.type === 'inbound-rtp' && isVideoReport(rtcReport)) {
        totals.inboundVideoStreams += 1
        totals.inboundBytes += numberValue(rtcReport, 'bytesReceived')
        totals.packetsLost += numberValue(rtcReport, 'packetsLost')
        totals.framesDecoded += numberValue(rtcReport, 'framesDecoded')
        totals.framesDropped += numberValue(rtcReport, 'framesDropped')
        totals.jitterMs ??= optionalMilliseconds(numberValue(rtcReport, 'jitter'))
      }

      if (rtcReport.type === 'outbound-rtp' && isVideoReport(rtcReport)) {
        totals.outboundVideoStreams += 1
        totals.outboundBytes += numberValue(rtcReport, 'bytesSent')
      }

      if (rtcReport.type === 'candidate-pair' && (rtcReport.nominated === true || rtcReport.selected === true)) {
        totals.roundTripTimeMs ??= optionalMilliseconds(numberValue(rtcReport, 'currentRoundTripTime'))
      }
    })
  }

  return totals
}

function bitrate(bytes: number, previousBytes: number, elapsedMilliseconds: number): number {
  if (elapsedMilliseconds <= 0 || bytes < previousBytes) return 0
  return Math.round(((bytes - previousBytes) * 8 * 1000) / elapsedMilliseconds)
}

export function startRtcDiagnostics() {
  if (!installPeerConnectionTracker()) {
    console.warn('[RTC diagnostics] RTCPeerConnection is unavailable')
    return
  }

  let previous: PreviousSample | undefined
  const report = async () => {
    const totals = await collectTotals()
    const timestamp = performance.now()
    const elapsedMilliseconds = previous ? timestamp - previous.timestamp : 0
    const sample: RtcDiagnosticsSample = {
      peerConnections: trackedConnections.size,
      inboundVideoStreams: totals.inboundVideoStreams,
      outboundVideoStreams: totals.outboundVideoStreams,
      inboundBitrateBps: previous ? bitrate(totals.inboundBytes, previous.inboundBytes, elapsedMilliseconds) : 0,
      outboundBitrateBps: previous ? bitrate(totals.outboundBytes, previous.outboundBytes, elapsedMilliseconds) : 0,
      packetsLost: totals.packetsLost,
      framesDecoded: totals.framesDecoded,
      framesDropped: totals.framesDropped,
      roundTripTimeMs: totals.roundTripTimeMs,
      jitterMs: totals.jitterMs,
    }

    previous = { inboundBytes: totals.inboundBytes, outboundBytes: totals.outboundBytes, timestamp }
    await invoke<void>('record_rtc_diagnostics', { sample }).catch(error => {
      console.warn('[RTC diagnostics] Failed to record sample:', error)
    })
  }

  void report()
  const interval = window.setInterval(() => void report(), SAMPLE_INTERVAL_MS)
  window.addEventListener('beforeunload', () => window.clearInterval(interval), { once: true })
}
