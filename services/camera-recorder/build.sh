#!/bin/bash
# Build and push camera-recorder Docker image

set -euo pipefail

REGISTRY="${REGISTRY:-ghcr.io/5dlabs}"
IMAGE="${REGISTRY}/camera-recorder"
TAG="${TAG:-latest}"

echo "🔨 Building camera-recorder"
echo "   Image: ${IMAGE}:${TAG}"
echo ""

# Build with BuildKit cache
docker buildx build \
  --platform linux/amd64 \
  --cache-from type=registry,ref=${IMAGE}:cache \
  --cache-to type=registry,ref=${IMAGE}:cache \
  -t ${IMAGE}:${TAG} \
  --push \
  .

echo ""
echo "✅ Build complete: ${IMAGE}:${TAG}"
echo ""
echo "Next steps:"
echo "  1. Create secrets: kubectl apply -f ../../manifests/secret.yaml"
echo "  2. Deploy: kubectl apply -f ../../manifests/"
echo "  3. Check logs: kubectl logs -n camera-system -l app=camera-recorder -f"
