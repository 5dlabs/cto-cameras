# CTO Cameras

Security camera recording system for continuous RTSP capture with S3-compatible storage.

## Features

- **Continuous Recording** - 24/7 RTSP stream capture from IP cameras
- **Segmented Storage** - 15-minute MP4 segments for easy management
- **S3 Upload** - Automatic upload to SeaweedFS/S3-compatible storage
- **Kubernetes Native** - Deploys via ArgoCD with full observability
- **Prometheus Metrics** - Real-time monitoring of recording status

## Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   Camera #1     │    │   Camera #2     │
│  192.168.1.97   │    │  192.168.1.13   │
└────────┬────────┘    └────────┬────────┘
         │   RTSP              │   RTSP
         └──────────┬──────────┘
                    ▼
         ┌──────────────────────┐
         │  Camera Recorder     │  (Rust)
         │  - FFmpeg capture    │
         │  - 15-min segments   │
         │  - S3 upload         │
         │  - Health checks     │
         └──────────┬───────────┘
                    │ S3 API
                    ▼
         ┌──────────────────────┐
         │  SeaweedFS (K8s)     │  (Persistent storage)
         └──────────────────────┘
```

## Quick Start

### Local Development

```bash
# Build the recorder
cd services/camera-recorder
cargo build --release

# Configure (copy and edit)
cp config.example.toml config.toml
# Edit config.toml with your camera URLs

# Run
./target/release/camera-recorder
```

### Kubernetes Deployment

1. **Deploy via ArgoCD:**
   ```bash
   kubectl apply -f manifests/argocd-application.yaml
   ```

2. **Configure Secrets:**
   Edit `manifests/secrets.yaml` with your camera credentials and apply:
   ```bash
   kubectl apply -f manifests/secrets.yaml
   ```

## Configuration

### Camera Configuration

Edit `services/camera-recorder/config.toml`:

```toml
[[cameras]]
id = "camera-1"
name = "Front Door"
url = "rtsp://admin:password@192.168.1.97:554/cam/realmonitor?channel=1&subtype=0"
enabled = true
resolution = "2560x1440"
fps = 15

[storage]
s3_endpoint = "http://seaweedfs-filer.seaweedfs.svc.cluster.local:8333"
s3_bucket = "camera-recordings"
s3_region = "us-east-1"
segment_duration_minutes = 15
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CAMERA_1_URL` | RTSP URL for camera 1 | - |
| `CAMERA_2_URL` | RTSP URL for camera 2 | - |
| `S3_ENDPOINT` | S3-compatible endpoint | - |
| `S3_BUCKET` | Storage bucket name | `camera-recordings` |
| `S3_ACCESS_KEY` | S3 access key | - |
| `S3_SECRET_KEY` | S3 secret key | - |
| `TEMP_DIR` | Local temp directory | `/tmp/camera-recordings` |

## Monitoring

### Prometheus Metrics

- `camera_recorder_segments_recorded_total` - Total segments recorded per camera
- `camera_recorder_segments_uploaded_total` - Total segments uploaded per camera
- `camera_recorder_upload_duration_seconds` - Upload duration histogram
- `camera_recorder_recording_active` - Whether recording is active (1/0)

### Health Endpoints

- `GET /health` - Liveness probe
- `GET /ready` - Readiness probe
- `GET /metrics` - Prometheus metrics

## Directory Structure

```
cto-cameras/
├── manifests/                    # Kubernetes manifests
│   ├── argocd-application.yaml   # ArgoCD app definition
│   ├── configmap.yaml            # Configuration
│   ├── deployment.yaml           # Deployment + Service
│   ├── namespace.yaml            # Namespace
│   └── secrets.yaml              # Secrets (template)
├── services/
│   ├── camera-recorder/          # Rust recorder service
│   └── playback-server/          # Python playback UI
├── scripts/                      # Helper scripts
└── .github/workflows/            # CI/CD
```

## License

MIT
