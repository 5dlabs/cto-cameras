# Camera Recorder Service

Rust-based continuous recording service for TP-Link Tapo C220 cameras with SeaweedFS storage backend.

## Features

- **Multi-camera support** - Records from multiple cameras simultaneously
- **Automatic segmentation** - 15-minute segments for easier management
- **S3-compatible storage** - Uploads to SeaweedFS (S3 API)
- **Resilient** - Auto-reconnect on stream failure, retry on upload failure
- **Cloud-native** - Designed for Kubernetes deployment
- **Observable** - Prometheus metrics and health endpoints

## Architecture

- **FFmpeg** - Handles RTSP streaming and video segmentation
- **Tokio async runtime** - Concurrent camera management
- **AWS SDK** - S3-compatible uploads to SeaweedFS
- **Axum** - HTTP server for metrics and health checks

## Camera Details

**Camera #1:**
- IP: 192.168.1.97
- Resolution: 2560x1440 (2K)
- FPS: ~15
- Codec: H.264

**Camera #2:**
- IP: 192.168.1.13
- Resolution: 2560x1440 (2K)
- FPS: ~14
- Codec: H.264

## Configuration

Configure via environment variables (see `config.example.toml` for structure):

**Required:**
- `S3_ENDPOINT` - SeaweedFS filer endpoint
- `S3_ACCESS_KEY_ID` - S3 access key
- `S3_SECRET_ACCESS_KEY` - S3 secret key
- `CAMERA1_RTSP_URL` - Camera #1 RTSP URL with credentials
- `CAMERA2_RTSP_URL` - Camera #2 RTSP URL with credentials

**Optional:**
- `METRICS_PORT` - Metrics server port (default: 9090)
- `S3_BUCKET` - S3 bucket name (default: camera-recordings)
- `TEMP_DIR` - Temporary storage (default: /tmp/camera-recordings)
- `MAX_CONCURRENT_UPLOADS` - Concurrent uploads (default: 4)

## Building

```bash
# Build binary
cargo build --release

# Build Docker image
docker build -t camera-recorder:latest .

# Build with BuildKit cache
docker buildx build --cache-from type=registry,ref=ghcr.io/5dlabs/camera-recorder:cache \
  --cache-to type=registry,ref=ghcr.io/5dlabs/camera-recorder:cache \
  -t ghcr.io/5dlabs/camera-recorder:latest \
  --push .
```

## Local Testing

```bash
# Set environment variables
export S3_ENDPOINT="http://localhost:8333"
export S3_ACCESS_KEY_ID="your-key"
export S3_SECRET_ACCESS_KEY="your-secret"
export CAMERA1_RTSP_URL="rtsp://5dlabs:pass@192.168.1.97:554/stream1"
export CAMERA2_RTSP_URL="rtsp://5dlabs2:pass@192.168.1.13:554/stream1"

# Run
cargo run --release
```

## Deployment

```bash
# Create secrets (use sealed-secrets or external-secrets in production)
kubectl create secret generic camera-recorder-secrets \
  -n camera-system \
  --from-literal=S3_ACCESS_KEY_ID=your-key \
  --from-literal=S3_SECRET_ACCESS_KEY=your-secret \
  --from-literal=CAMERA1_RTSP_URL=rtsp://... \
  --from-literal=CAMERA2_RTSP_URL=rtsp://...

# Apply manifests
kubectl apply -f manifests/

# Or deploy via ArgoCD
kubectl apply -f manifests/argocd-application.yaml
```

## Monitoring

**Metrics endpoint:** `http://camera-recorder.camera-system.svc:9090/metrics`

**Health checks:**
- `/health` - Service is running
- `/ready` - All cameras connected

**Prometheus queries:**
```promql
# Camera connection status
camera_stream_connected{camera_id="camera-1"}

# Segments uploaded
rate(camera_segments_uploaded_total[5m])

# Upload failures
rate(camera_upload_failures_total[5m])
```

## Storage Structure

```
bucket: camera-recordings
├── camera-1/
│   ├── 20251214/
│   │   ├── 01-30-00_camera-1.mp4
│   │   ├── 01-45-00_camera-1.mp4
│   │   └── 02-00-00_camera-1.mp4
│   └── 20251215/
│       └── ...
└── camera-2/
    └── ...
```

## Storage Requirements

- **Per camera:** ~2 GB/hour @ 2K resolution
- **Both cameras:** ~48 GB/day, ~1.4 TB/month
- **Current SeaweedFS capacity:** 200 GB (2 volume servers × 100 GB)
- **Retention:** ~4 days before needing expansion

## Troubleshooting

**Camera not connecting:**
```bash
# Check logs
kubectl logs -n camera-system -l app=camera-recorder -f

# Verify RTSP URL
kubectl exec -n camera-system deploy/camera-recorder -- \
  ffmpeg -rtsp_transport tcp -i "$CAMERA1_RTSP_URL" -t 1 -f null -
```

**Upload failures:**
```bash
# Check SeaweedFS status
kubectl get pods -n seaweedfs

# Test S3 API
kubectl run -it --rm s3-test --image=amazon/aws-cli -n camera-system -- \
  s3 ls s3://camera-recordings --endpoint-url=http://seaweedfs-filer.seaweedfs.svc:8333
```

## Development Status

- [x] Project structure created
- [x] Dependencies configured
- [x] Configuration management
- [x] Main orchestrator
- [x] Camera recorder (basic)
- [x] S3 client
- [x] Upload logic
- [x] Metrics server
- [x] Dockerfile
- [x] Kubernetes manifests
- [ ] Complete FFmpeg process manager
- [ ] Implement segment rotation logic
- [ ] Full Prometheus metrics
- [ ] Integration tests
- [ ] Build and deploy

## Next Steps

1. Complete FFmpeg process manager with segment detection
2. Implement upload queue and background uploader
3. Add full Prometheus metrics
4. Build Docker image
5. Create SeaweedFS credentials
6. Deploy to cluster
7. Validate 24-hour recording test
