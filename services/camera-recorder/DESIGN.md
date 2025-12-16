# Camera Recorder Service - Design Document

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     camera-recorder Service                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Main Orchestrator (Tokio Runtime)             │ │
│  │  - Manages camera recorders                                │ │
│  │  - Health monitoring                                       │ │
│  │  - Graceful shutdown                                       │ │
│  │  - Metrics aggregation                                     │ │
│  └────────────────┬─────────────────┬─────────────────────────┘ │
│                   │                 │                            │
│  ┌────────────────▼──────────────┐  ┌──▼──────────────────────┐ │
│  │   Camera Recorder #1          │  │  Camera Recorder #2      │ │
│  │   (192.168.1.97)              │  │  (192.168.1.13)          │ │
│  │  ┌─────────────────────────┐  │  │ ┌─────────────────────┐ │ │
│  │  │  RTSP Client            │  │  │ │  RTSP Client        │ │ │
│  │  │  - TCP transport        │  │  │ │  - TCP transport    │ │ │
│  │  │  - Auto-reconnect       │  │  │ │  - Auto-reconnect   │ │ │
│  │  └───────────┬─────────────┘  │  │ └──────────┬──────────┘ │ │
│  │              │                 │  │            │            │ │
│  │  ┌───────────▼─────────────┐  │  │ ┌──────────▼──────────┐ │ │
│  │  │  FFmpeg Process         │  │  │ │  FFmpeg Process     │ │ │
│  │  │  - H.264 copy           │  │  │ │  - H.264 copy       │ │ │
│  │  │  - AAC audio            │  │  │ │  - AAC audio        │ │ │
│  │  │  - 15-min segments      │  │  │ │  - 15-min segments  │ │ │
│  │  └───────────┬─────────────┘  │  │ └──────────┬──────────┘ │ │
│  │              │                 │  │            │            │ │
│  │  ┌───────────▼─────────────┐  │  │ ┌──────────▼──────────┐ │ │
│  │  │  Segment Manager        │  │  │ │  Segment Manager    │ │ │
│  │  │  - Rotation timer       │  │  │ │  - Rotation timer   │ │ │
│  │  │  - Upload queue         │  │  │ │  - Upload queue     │ │ │
│  │  └───────────┬─────────────┘  │  │ └──────────┬──────────┘ │ │
│  └──────────────┼─────────────────┘  └────────────┼────────────┘ │
│                 │                                  │              │
│                 └──────────────┬───────────────────┘              │
│                                │                                  │
│  ┌─────────────────────────────▼────────────────────────────┐   │
│  │              S3 Uploader (SeaweedFS Client)              │   │
│  │  - Async upload queue                                    │   │
│  │  - Retry logic with backoff                              │   │
│  │  - S3-compatible API (AWS SDK)                           │   │
│  │  - Bucket: camera-recordings                             │   │
│  │  - Path: camera-{id}/{date}/{timestamp}.mp4              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Metrics & Health Server (Axum)              │   │
│  │  - /metrics (Prometheus)                                 │   │
│  │  - /health (Kubernetes probes)                           │   │
│  │  - /ready (Readiness check)                              │   │
│  └──────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   SeaweedFS S3 API    │
                    │   (seaweedfs-filer)   │
                    │   Port: 8333          │
                    └───────────────────────┘
```

## Component Details

### 1. Main Orchestrator
**Responsibilities:**
- Initialize camera recorders from config
- Monitor recorder health
- Handle graceful shutdown (SIGTERM/SIGINT)
- Coordinate S3 uploader
- Serve metrics endpoint

**Key Features:**
- Tokio async runtime
- Structured logging with tracing
- Signal handling for clean shutdown

### 2. Camera Recorder
**Responsibilities:**
- Maintain RTSP connection
- Spawn FFmpeg process for recording
- Monitor FFmpeg health
- Rotate segments every 15 minutes
- Queue completed segments for upload

**Key Features:**
- Auto-reconnect on stream failure
- Process supervision (restart FFmpeg if crashes)
- Segment naming: `{camera_id}/{date}/{HH-MM-SS}_15min.mp4`
- Temporary local storage before upload

### 3. FFmpeg Integration
**Process Command:**
```bash
ffmpeg -rtsp_transport tcp \
  -i "rtsp://user:pass@ip:554/stream1" \
  -c:v copy \
  -c:a aac \
  -f segment \
  -segment_time 900 \
  -segment_format mp4 \
  -strftime 1 \
  -reset_timestamps 1 \
  "recording_%Y%m%d_%H%M%S.mp4"
```

**Parameters:**
- `-rtsp_transport tcp`: Reliable TCP transport
- `-c:v copy`: Copy H.264 stream (no re-encoding)
- `-c:a aac`: Convert audio to AAC
- `-f segment`: Enable segmentation
- `-segment_time 900`: 15-minute segments (900 seconds)
- `-reset_timestamps 1`: Reset timestamps per segment

### 4. S3 Uploader
**Responsibilities:**
- Upload completed segments to SeaweedFS
- Retry failed uploads (exponential backoff)
- Clean up local files after successful upload
- Track upload metrics

**Storage Structure:**
```
bucket: camera-recordings
├── camera-1/
│   ├── 2025-12-13/
│   │   ├── 14-30-00_15min.mp4
│   │   ├── 14-45-00_15min.mp4
│   │   └── 15-00-00_15min.mp4
│   └── 2025-12-14/
│       └── ...
└── camera-2/
    └── ...
```

**SeaweedFS Configuration:**
- **Endpoint:** `http://seaweedfs-filer.seaweedfs.svc.cluster.local:8333`
- **Bucket:** `camera-recordings`
- **Authentication:** Via AWS SDK with access key/secret

### 5. Metrics & Monitoring
**Prometheus Metrics:**
- `camera_stream_connected{camera_id}` - Connection status (0/1)
- `camera_segments_recorded_total{camera_id}` - Total segments recorded
- `camera_segments_uploaded_total{camera_id}` - Total segments uploaded
- `camera_upload_failures_total{camera_id}` - Upload failures
- `camera_segment_upload_duration_seconds{camera_id}` - Upload duration
- `camera_ffmpeg_restarts_total{camera_id}` - FFmpeg restart count
- `camera_recording_bytes_total{camera_id}` - Total bytes recorded

**Health Endpoints:**
- `/health` - Overall service health
- `/ready` - Readiness (both cameras connected)
- `/metrics` - Prometheus metrics

## Configuration

**Config File:** `/etc/camera-recorder/config.toml`

```toml
[service]
metrics_port = 9090

[storage]
endpoint = "http://seaweedfs-filer.seaweedfs.svc.cluster.local:8333"
bucket = "camera-recordings"
region = "us-east-1"  # Default, not really used by SeaweedFS
access_key_id = "${S3_ACCESS_KEY_ID}"
secret_access_key = "${S3_SECRET_ACCESS_KEY}"

[[cameras]]
id = "camera-1"
name = "Camera 1"
rtsp_url = "rtsp://5dlabs:PASSWORD@192.168.1.97:554/stream1"
segment_duration_secs = 900  # 15 minutes

[[cameras]]
id = "camera-2"
name = "Camera 2"
rtsp_url = "rtsp://5dlabs2:PASSWORD@192.168.1.13:554/stream1"
segment_duration_secs = 900  # 15 minutes

[recording]
# Temporary local storage before upload
temp_dir = "/tmp/camera-recordings"
# Keep segments locally until uploaded + X minutes (for redundancy)
local_retention_minutes = 60
# Video codec (copy = no re-encoding)
video_codec = "copy"
# Audio codec (aac = compatible with MP4)
audio_codec = "aac"

[upload]
# Concurrent uploads
max_concurrent = 4
# Retry configuration
max_retries = 5
retry_backoff_secs = 5
```

## Module Structure

```
camera-recorder/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, orchestrator
│   ├── config.rs               # Configuration management
│   ├── camera/
│   │   ├── mod.rs              # Camera recorder module
│   │   ├── recorder.rs         # Camera recorder implementation
│   │   └── ffmpeg.rs           # FFmpeg process manager
│   ├── storage/
│   │   ├── mod.rs              # Storage module
│   │   ├── s3_client.rs        # SeaweedFS S3 client
│   │   └── uploader.rs         # Upload queue and retry logic
│   ├── metrics/
│   │   ├── mod.rs              # Metrics module
│   │   └── server.rs           # Metrics HTTP server
│   └── health.rs               # Health check logic
└── tests/
    └── integration_tests.rs
```

## Error Handling Strategy

**FFmpeg Process Failures:**
- Restart immediately with exponential backoff
- Max 5 retries before alerting
- Log stderr for debugging

**RTSP Connection Failures:**
- Retry every 30 seconds
- Never give up (continuous retry)
- Emit metrics for monitoring

**Upload Failures:**
- Retry with exponential backoff (5s, 10s, 20s, 40s, 80s)
- Keep segments locally if upload fails
- Alert after 5 failures

**Graceful Shutdown:**
- SIGTERM: Stop recording new segments
- Finish current segment
- Upload pending segments
- Exit cleanly

## Resource Requirements

**Per-Instance:**
- **CPU:** 500m-1000m (FFmpeg encoding)
- **Memory:** 512Mi-1Gi
- **Storage:** 5-10Gi (temp storage for segments before upload)
- **Network:** ~1 Mbps per camera (egress to storage)

**For 2 Cameras:**
- **CPU:** 1-2 cores
- **Memory:** 1-2Gi
- **Storage:** 10-20Gi temp

## Deployment Strategy

**Single Pod:**
- One pod manages both cameras
- Simpler coordination
- Lower resource overhead

**Replica Strategy:**
- 1 replica (StatefulSet)
- PVC for temp storage
- NodeSelector to pin to specific node if needed

## Monitoring & Alerts

**Critical Alerts:**
- Camera offline for > 5 minutes
- Upload failures > 10 in 1 hour
- Disk usage > 80%
- FFmpeg restarts > 5 in 10 minutes

**Warning Alerts:**
- Camera reconnects > 3 in 1 hour
- Upload latency > 5 minutes
- Segment size anomaly (too large/small)

## Storage Capacity Planning

**Per Camera:**
- ~2 GB/hour at 2K resolution
- 15-min segments = ~500 MB each
- 96 segments per day = ~48 GB/day
- Both cameras = ~96 GB/day

**SeaweedFS:**
- Current capacity: 100 GB per volume server × 2 = 200 GB total
- ~2 days retention before needing expansion
- **Recommendation:** Increase volume size or add lifecycle policies

## Next Steps

1. Implement core modules
2. Build Docker image with FFmpeg
3. Create Kubernetes manifests
4. Create SeaweedFS bucket
5. Deploy via ArgoCD
6. Test 24-hour continuous recording
