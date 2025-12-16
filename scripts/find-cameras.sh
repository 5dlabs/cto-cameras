#!/bin/bash
# Find Tapo cameras on the local network

set -euo pipefail

echo "🔍 Scanning for Tapo cameras on the network..."
echo ""

# Detect local network CIDR
if command -v ipconfig &> /dev/null; then
    # macOS
    LOCAL_IP=$(ipconfig getifaddr en0 || ipconfig getifaddr en1 || echo "")
elif command -v ip &> /dev/null; then
    # Linux
    LOCAL_IP=$(ip route get 1 | awk '{print $7; exit}')
else
    echo "❌ Cannot detect local IP address"
    exit 1
fi

if [ -z "$LOCAL_IP" ]; then
    echo "❌ No local IP address found. Are you connected to WiFi?"
    exit 1
fi

# Extract network portion (assuming /24 network)
NETWORK=$(echo "$LOCAL_IP" | cut -d'.' -f1-3)
echo "Local IP: $LOCAL_IP"
echo "Scanning network: $NETWORK.0/24"
echo ""

# Check if nmap is installed
if ! command -v nmap &> /dev/null; then
    echo "⚠️  nmap is not installed. Install it with:"
    echo "   macOS: brew install nmap"
    echo "   Linux: sudo apt install nmap"
    exit 1
fi

# Scan for RTSP ports (Tapo cameras use port 554)
echo "📡 Scanning for RTSP servers (port 554)..."
echo ""

nmap -p 554 --open "$NETWORK.0/24" | grep -E "Nmap scan report|554/tcp"

echo ""
echo "✅ Scan complete!"
echo ""
echo "Next steps:"
echo "1. Note the IP addresses with port 554 open"
echo "2. Test RTSP streams with: ffplay 'rtsp://username:password@IP:554/stream1'"
echo "3. Update docs/SETUP_CHECKLIST.md with IP addresses"
