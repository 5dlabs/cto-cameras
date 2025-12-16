#!/bin/bash
# Test RTSP stream connectivity

set -euo pipefail

usage() {
    echo "Usage: $0 <camera-ip> <username> <password>"
    echo ""
    echo "Example:"
    echo "  $0 192.168.1.100 admin mysecretpass"
    echo ""
    echo "This script will:"
    echo "  1. Test RTSP connectivity"
    echo "  2. Record 30 seconds of video"
    echo "  3. Display stream information"
    exit 1
}

if [ $# -ne 3 ]; then
    usage
fi

CAMERA_IP="$1"
USERNAME="$2"
PASSWORD="$3"
RTSP_URL="rtsp://${USERNAME}:${PASSWORD}@${CAMERA_IP}:554/stream1"
OUTPUT_FILE="test-recording-$(date +%Y%m%d-%H%M%S).mp4"

echo "🎥 Testing RTSP stream for camera at $CAMERA_IP"
echo ""

# Check if ffmpeg/ffprobe is installed
if ! command -v ffmpeg &> /dev/null; then
    echo "❌ ffmpeg is not installed. Install it with:"
    echo "   macOS: brew install ffmpeg"
    echo "   Linux: sudo apt install ffmpeg"
    exit 1
fi

# Test 1: Get stream info
echo "📊 Getting stream information..."
if ffprobe -v quiet -print_format json -show_streams "$RTSP_URL" > stream-info.json 2>&1; then
    echo "✅ Successfully connected to RTSP stream"
    
    # Extract key info
    if command -v jq &> /dev/null; then
        echo ""
        echo "Stream details:"
        jq -r '.streams[] | select(.codec_type=="video") | "  Resolution: \(.width)x\(.height)\n  Codec: \(.codec_name)\n  FPS: \(.r_frame_rate)"' stream-info.json
    fi
else
    echo "❌ Failed to connect to RTSP stream"
    echo "   Please check:"
    echo "   - Camera IP address is correct"
    echo "   - Username/password are correct"
    echo "   - RTSP is enabled in Tapo app"
    echo "   - Camera is on the same network"
    exit 1
fi

# Test 2: Record 30 seconds
echo ""
echo "🎬 Recording 30 seconds of video to $OUTPUT_FILE..."
if ffmpeg -y -rtsp_transport tcp -i "$RTSP_URL" -t 30 -c copy "$OUTPUT_FILE" > /dev/null 2>&1; then
    echo "✅ Recording successful!"
    
    # Get file size
    FILE_SIZE=$(du -h "$OUTPUT_FILE" | cut -f1)
    echo "   File size: $FILE_SIZE"
    echo "   Location: $PWD/$OUTPUT_FILE"
else
    echo "❌ Recording failed"
    exit 1
fi

# Test 3: Verify file
echo ""
echo "🔍 Verifying recorded file..."
if ffprobe -v error "$OUTPUT_FILE" > /dev/null 2>&1; then
    echo "✅ File is valid and playable"
else
    echo "❌ File verification failed"
    exit 1
fi

echo ""
echo "✅ All tests passed!"
echo ""
echo "Next steps:"
echo "  1. Play the test file: vlc $OUTPUT_FILE"
echo "  2. Update docs/SETUP_CHECKLIST.md with camera details"
echo "  3. Repeat for the second camera"

# Cleanup
rm -f stream-info.json
