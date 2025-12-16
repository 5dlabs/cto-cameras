# Camera Recording System - Deployment Guide

## Pre-Deployment Checklist

- [x] Both cameras tested and verified (192.168.1.97, 192.168.1.13)
- [x] Rust service implemented and compiled
- [x] Clippy passed with no warnings
- [x] Kubernetes manifests created
- [ ] Docker image built and pushed
- [ ] SeaweedFS S3 credentials created
- [ ] Kubernetes secrets created
- [ ] Deployed to cluster

## Step 1: Build Docker Image

```bash
cd /Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system/services/camera-recorder

# Option A: Simple build
docker build -t ghcr.io/5dlabs/camera-recorder:latest .

# Option B: Build with BuildKit cache (faster rebuilds)
docker buildx build \
  --platform linux/amd64 \
  --cache-from type=registry,ref=ghcr.io/5dlabs/camera-recorder:cache \
  --cache-to type=registry,ref=ghcr.io/5dlabs/camera-recorder:cache \
  -t ghcr.io/5dlabs/camera-recorder:latest \
  --push .
```

**Note:** You need to be logged into ghcr.io:
```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin
```

## Step 2: Create SeaweedFS S3 Credentials

```bash
# Access SeaweedFS master
kubectl exec -it -n seaweedfs seaweedfs-master-0 -- /bin/sh

# Inside the pod, create S3 credentials
/usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 << 'WEED'
s3.configure -user camera-recorder -actions Read,Write,List,Delete
WEED

# Note the access_key_id and secret_access_key returned
# Exit the pod
```

Alternative using weed shell directly:
```bash
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='s3.configure -user camera-recorder -actions Read,Write,List,Delete'
```

## Step 3: Create Storage Bucket

```bash
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='s3.bucket.create -name camera-recordings'
```

Verify bucket exists:
```bash
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='s3.bucket.list'
```

## Step 4: Create Kubernetes Secrets

```bash
# Create secret with camera credentials and S3 credentials
kubectl create secret generic camera-recorder-secrets \
  -n camera-system \
  --from-literal=S3_ACCESS_KEY_ID='<from-step-2>' \
  --from-literal=S3_SECRET_ACCESS_KEY='<from-step-2>' \
  --from-literal=CAMERA1_RTSP_URL='rtsp://5dlabs:jechaz-0mujba-diwwYh@192.168.1.97:554/stream1' \
  --from-literal=CAMERA2_RTSP_URL='rtsp://5dlabs2:qyfpev-1gukko-Qowjej@192.168.1.13:554/stream1'
```

**For production:** Use Sealed Secrets or external-secrets instead of plain Secrets.

## Step 5: Deploy to Kubernetes

### Option A: Direct kubectl

```bash
cd /Users/jonathonfritz/code/work-projects/5dlabs/cto-cameras/projects/camera-system

# Apply all manifests
kubectl apply -f manifests/namespace.yaml
kubectl apply -f manifests/configmap.yaml
kubectl apply -f manifests/deployment.yaml

# Verify deployment
kubectl get pods -n camera-system
```

### Option B: ArgoCD (Recommended)

```bash
# Apply ArgoCD Application
kubectl apply -f manifests/argocd-application.yaml

# Watch sync status
kubectl get application -n argocd camera-recorder -w

# Or use argocd CLI
argocd app sync camera-recorder
argocd app wait camera-recorder
```

## Step 6: Verify Deployment

### Check Pods

```bash
# Watch pod startup
kubectl get pods -n camera-system -w

# Should see: camera-recorder-xxx Running and Ready 1/1
```

### Check Logs

```bash
# Follow logs
kubectl logs -n camera-system -l app=camera-recorder -f

# Should see:
# - "Starting camera recorder service"
# - "Camera connected, recording started" (for each camera)
# - "Segment completed, queuing for upload"
# - "Upload successful"
```

### Test Health Endpoints

```bash
# Port-forward metrics
kubectl port-forward -n camera-system svc/camera-recorder 9090:9090

# In another terminal:
curl http://localhost:9090/health
# Expected: OK

curl http://localhost:9090/ready
# Expected: READY (when both cameras connected)

curl http://localhost:9090/metrics
# Expected: Prometheus metrics output
```

## Step 7: Verify Recordings in SeaweedFS

```bash
# List bucket contents
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='fs.ls /camera-recordings/'

# Should see camera-1 and camera-2 directories

# List camera-1 recordings
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='fs.ls /camera-recordings/camera-1/'

# Should see date directories (YYYYMMDD)

# List today's recordings
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 -filer=seaweedfs-filer:8888 \
  -shell.command='fs.ls /camera-recordings/camera-1/20251214/'

# Should see .mp4 files
```

## Step 8: Monitor with Prometheus

```bash
# Port-forward to Prometheus
kubectl port-forward -n observability svc/prometheus-server 9090:80

# Query camera metrics
curl -g 'http://localhost:9090/api/v1/query?query=camera_stream_connected'
curl -g 'http://localhost:9090/api/v1/query?query=camera_segments_recorded_total'
curl -g 'http://localhost:9090/api/v1/query?query=camera_segments_uploaded_total'
```

## Troubleshooting

### Cameras Not Connecting

```bash
# Check if cameras are reachable from pod
kubectl exec -n camera-system deploy/camera-recorder -- \
  curl -I --max-time 5 http://192.168.1.97

# Test RTSP from pod (if ffmpeg is available)
kubectl exec -n camera-system deploy/camera-recorder -- \
  ffmpeg -rtsp_transport tcp -i "$CAMERA1_RTSP_URL" -t 1 -f null -
```

If cameras aren't reachable, you may need:
- `hostNetwork: true` in deployment
- Or ensure Kubernetes CNI can route to 192.168.1.0/24

### Upload Failures

```bash
# Check SeaweedFS health
kubectl get pods -n seaweedfs

# Test S3 API from camera-system namespace
kubectl run -it --rm s3-test --image=amazon/aws-cli -n camera-system -- \
  s3 ls s3://camera-recordings \
  --endpoint-url=http://seaweedfs-filer.seaweedfs.svc.cluster.local:8333 \
  --region us-east-1
```

### Pod Crashing

```bash
# Get pod logs
kubectl logs -n camera-system -l app=camera-recorder --previous

# Describe pod to see events
kubectl describe pod -n camera-system -l app=camera-recorder

# Check resource usage
kubectl top pod -n camera-system
```

### Storage Full

```bash
# Check SeaweedFS volume usage
kubectl exec -n seaweedfs seaweedfs-master-0 -- \
  /usr/bin/weed shell -master=localhost:9333 \
  -shell.command='volume.list'

# Expand volume if needed (see SeaweedFS docs)
```

## Monitoring & Alerts

### Key Metrics to Monitor

- **camera_stream_connected** - Should be 1 for both cameras
- **camera_segments_recorded_total** - Should increase every 15 minutes
- **camera_segments_uploaded_total** - Should match segments_recorded
- **camera_upload_failures_total** - Should stay at 0

### Recommended Alerts

**Critical:**
- Camera disconnected for > 5 minutes
- Upload failure rate > 10% over 1 hour
- Temp storage usage > 80%

**Warning:**
- Camera reconnects > 3 per hour
- Upload latency > 5 minutes per segment

## Maintenance

### Viewing Recordings

To download a recording from SeaweedFS:

```bash
# Port-forward S3 API
kubectl port-forward -n seaweedfs svc/seaweedfs-filer 8333:8333

# Use awscli to download
aws s3 cp s3://camera-recordings/camera-1/20251214/013000_camera-1.mp4 ./recording.mp4 \
  --endpoint-url http://localhost:8333 \
  --region us-east-1
```

### Cleanup Old Recordings

```bash
# Delete recordings older than 7 days (manual)
# Or implement S3 lifecycle policy in SeaweedFS
```

### Updating Service

```bash
# Make code changes
cd services/camera-recorder

# Rebuild image
./build.sh

# Restart deployment
kubectl rollout restart deployment/camera-recorder -n camera-system

# Watch rollout
kubectl rollout status deployment/camera-recorder -n camera-system
```

## Performance Tuning

### If CPU is high:
- Cameras are already using H.264 with copy (no re-encoding)
- Check if audio encoding is causing issues
- Consider lowering segment duration

### If uploads are slow:
- Increase `MAX_CONCURRENT_UPLOADS`
- Check network bandwidth to SeaweedFS
- Verify SeaweedFS volume server health

### If memory is high:
- Check temp storage cleanup is working
- Reduce segment duration
- Lower max_concurrent uploads

## Next Steps After Deployment

1. Monitor for 1 hour - verify segments uploading
2. Check one full 15-minute cycle completes
3. Verify storage organization in SeaweedFS
4. Run 24-hour stability test
5. Set up Grafana dashboard
6. Configure alerts in Prometheus
7. Document runbook for operations

---

**Service Status:** ✅ Ready to deploy!
