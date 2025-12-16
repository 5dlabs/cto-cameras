# Camera Recording System - LIVE AND RUNNING!

## Current Status: ✅ OPERATIONAL

**Recording Since:** December 14, 2025 at 02:44 AM  
**Total Recorded:** 881 MB (26 segments)  
**Cameras Active:** 2/2  
**Storage:** SeaweedFS (camera-recordings bucket)

---

## Access Your Recordings

### Web Interface (Recommended)

**URL:** http://localhost:8001

- Browse all recordings by camera
- Play videos in browser
- Auto-refreshes every 10 seconds
- Shows file sizes and timestamps

### Direct File Access

Recordings are stored at: `/tmp/camera-recordings/`

```bash
# Open recordings folder
open /tmp/camera-recordings/

# Play latest Camera 1 recording
open /tmp/camera-recordings/camera-1/*.mp4

# Play latest Camera 2 recording
open /tmp/camera-recordings/camera-2/*.mp4
```

---

## Running Services

### 1. Camera Recorder (Rust)
**Location:** `/Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/camera-recorder/target/release/camera-recorder`  
**Logs:** `/tmp/camera-recorder.log`  
**Metrics:** http://localhost:9090/metrics

**What it does:**
- Records from both cameras simultaneously
- Creates 15-minute segments
- Uploads to SeaweedFS automatically
- Auto-reconnects on failures

### 2. Playback Server (Python)
**Location:** `/Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/playback-server/simple-server.py`  
**URL:** http://localhost:8001  
**Logs:** `/tmp/playback-server.log`

**What it does:**
- Serves web interface for browsing recordings
- Streams MP4 files for playback
- Updates every 10 seconds

### 3. SeaweedFS Port-Forward
**Command:** `kubectl port-forward -n seaweedfs svc/seaweedfs-s3 8333:8333`  
**Logs:** `/tmp/seaweedfs-pf.log`

**What it does:**
- Forwards SeaweedFS S3 API to localhost:8333
- Enables uploads from Mac to cluster storage

---

## Managing the System

### Check Status

```bash
# Are services running?
ps aux | grep camera-recorder | grep -v grep
ps aux | grep simple-server | grep -v grep
ps aux | grep "kubectl port-forward.*seaweedfs" | grep -v grep

# Watch logs
tail -f /tmp/camera-recorder.log

# Check metrics
curl http://localhost:9090/metrics | grep camera_

# List all recordings
ls -lh /tmp/camera-recordings/camera-*/*.mp4
```

### Stop Services

```bash
# Stop camera recorder
pkill camera-recorder

# Stop playback server
pkill -f "python3.*simple-server"

# Stop port-forward
pkill -f "kubectl port-forward.*seaweedfs"
```

### Restart Services

```bash
# 1. Start port-forward
kubectl port-forward -n seaweedfs svc/seaweedfs-s3 8333:8333 > /tmp/seaweedfs-pf.log 2>&1 &

# 2. Start camera recorder
cd /Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/camera-recorder
S3_ENDPOINT="http://127.0.0.1:8333" \
S3_ACCESS_KEY_ID="Epjm8T2IfRXQI5Cm" \
S3_SECRET_ACCESS_KEY="idni3vSg54jWli6HyX8bLe7F8ro682M6" \
S3_BUCKET="camera-recordings" \
CAMERA1_RTSP_URL="rtsp://5dlabs:jechaz-0mujba-diwwYh@192.168.1.97:554/stream1" \
CAMERA2_RTSP_URL="rtsp://5dlabs2:qyfpev-1gukko-Qowjej@192.168.1.13:554/stream1" \
RUST_LOG="info,camera_recorder=debug" \
nohup target/release/camera-recorder > /tmp/camera-recorder.log 2>&1 &

# 3. Start playback server
cd /Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/playback-server
PORT=8001 python3 simple-server.py &
```

---

## Camera Details

### Camera #1
- **IP:** 192.168.1.97
- **Username:** 5dlabs  
- **Password:** jechaz-0mujba-diwwYh
- **Resolution:** 2560x1440 @ 15 FPS
- **RTSP URL:** `rtsp://5dlabs:jechaz-0mujba-diwwYh@192.168.1.97:554/stream1`

### Camera #2
- **IP:** 192.168.1.13
- **Username:** 5dlabs2
- **Password:** qyfpev-1gukko-Qowjej
- **Resolution:** 2560x1440 @ 14 FPS
- **RTSP URL:** `rtsp://5dlabs2:qyfpev-1gukko-Qowjej@192.168.1.13:554/stream1`

---

## Storage Details

### SeaweedFS Configuration
- **Endpoint:** http://seaweedfs-s3.seaweedfs.svc.cluster.local:8333
- **Bucket:** camera-recordings
- **Access Key:** Epjm8T2IfRXQI5Cm
- **Secret Key:** idni3vSg54jWli6HyX8bLe7F8ro682M6

### Storage Organization

```
camera-recordings/
├── camera-1/
│   └── 20251214/
│       ├── 024401_camera-1.mp4
│       ├── 025900_camera-1.mp4
│       ├── 031400_camera-1.mp4
│       └── ... (24 total)
└── camera-2/
    └── 20251214/
        ├── 024401_camera-2.mp4
        └── 025900_camera-2.mp4
```

### Access SeaweedFS Storage

```bash
# List all recordings in SeaweedFS
AWS_ACCESS_KEY_ID="Epjm8T2IfRXQI5Cm" \
AWS_SECRET_ACCESS_KEY="idni3vSg54jWli6HyX8bLe7F8ro682M6" \
aws s3 ls s3://camera-recordings/camera-1/20251214/ \
--endpoint-url=http://127.0.0.1:8333 \
--region us-east-1

# Download a specific recording
AWS_ACCESS_KEY_ID="Epjm8T2IfRXQI5Cm" \
AWS_SECRET_ACCESS_KEY="idni3vSg54jWli6HyX8bLe7F8ro682M6" \
aws s3 cp s3://camera-recordings/camera-1/20251214/032901_camera-1.mp4 ./recording.mp4 \
--endpoint-url=http://127.0.0.1:8333 \
--region us-east-1
```

---

## Recording Specifications

**Video Format:**
- Codec: H.264 (High Profile)
- Resolution: 2560x1440 (2K)
- Frame Rate: 14-15 FPS
- Bitrate: ~350-400 kbps

**Audio Format:**
- Codec: AAC-LC
- Sample Rate: 8000 Hz
- Channels: Mono
- Bitrate: ~35-40 kbps

**Segmentation:**
- Duration: 15 minutes (900 seconds)
- Size: ~30-40 MB per segment (varies by content)
- Format: MP4 container
- Naming: `YYYYMMDD_HHMMSS_camera-id.mp4`

---

## Troubleshooting

### Cameras Not Recording?

```bash
# Check logs
tail -50 /tmp/camera-recorder.log

# Verify service is running
ps aux | grep camera-recorder | grep -v grep

# Test camera streams manually
ffmpeg -rtsp_transport tcp -i "rtsp://5dlabs:jechaz-0mujba-diwwYh@192.168.1.97:554/stream1" -t 5 -c copy test.mp4
```

### Uploads Failing?

```bash
# Check port-forward
ps aux | grep "kubectl port-forward.*seaweedfs" | grep -v grep

# Test S3 connection
curl -s http://127.0.0.1:8333/

# Restart port-forward if needed
pkill -f "kubectl port-forward.*seaweedfs"
kubectl port-forward -n seaweedfs svc/seaweedfs-s3 8333:8333 &
```

### Web Interface Not Loading?

```bash
# Check if server is running
ps aux | grep simple-server | grep -v grep

# Restart playback server
cd /Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/playback-server
PORT=8001 python3 simple-server.py &

# Open in browser
open http://localhost:8001
```

---

## Quick Commands Reference

```bash
# View recordings in web browser
open http://localhost:8001

# Watch live recording logs
tail -f /tmp/camera-recorder.log

# Play a recording
open /tmp/camera-recordings/camera-1/*.mp4

# Check current recording sizes
ls -lh /tmp/camera-recordings/camera-*/*.mp4 | tail -4

# Check how much has been recorded
du -sh /tmp/camera-recordings/

# Monitor uploads
tail -f /tmp/camera-recorder.log | grep "Upload successful"

# Check metrics
curl -s http://localhost:9090/metrics | grep camera_
```

---

## Future Enhancements

### Phase 2 (AI Integration)
- Real-time pose estimation
- Movement classification
- Interactive AI feedback
- Session analytics

### Phase 3 (Advanced Features)
- HLS live streaming
- Motion detection alerts
- Automated highlights
- Mobile app access
- Cloud backup
- AI-powered search ("show me squats from yesterday")

---

## Project Files

**Worktree:** `/Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras`  
**Branch:** `project/camera-recording-system`

**Documentation:**
- `SYSTEM_RUNNING.md` - This file (operational guide)
- `IMPLEMENTATION_COMPLETE.md` - Implementation summary
- `DEPLOYMENT_GUIDE.md` - Deployment instructions
- `services/camera-recorder/README.md` - Service documentation
- `services/camera-recorder/DESIGN.md` - Architecture details

**Service Code:**
- `services/camera-recorder/` - Rust recording service
- `services/playback-server/` - Python web interface

---

## Support

**Your system is recording continuously!**  
All recordings are saved locally and being uploaded to SeaweedFS storage in your Kubernetes cluster.

**Need help?** Check the logs:
- Camera recorder: `/tmp/camera-recorder.log`
- Playback server: `/tmp/playback-server.log`  
- SeaweedFS port-forward: `/tmp/seaweedfs-pf.log`

---

**System Status:** 🟢 OPERATIONAL  
**Last Updated:** December 14, 2025 at 08:55 AM
