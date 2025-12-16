#!/usr/bin/env python3
"""
Simple HTTP server to browse and stream camera recordings from SeaweedFS
"""

import os
import json
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
import boto3
from botocore.client import Config

# SeaweedFS S3 configuration
S3_ENDPOINT = os.getenv('S3_ENDPOINT', 'http://localhost:8333')
S3_ACCESS_KEY = os.getenv('S3_ACCESS_KEY_ID', 'Epjm8T2IfRXQI5Cm')
S3_SECRET_KEY = os.getenv('S3_SECRET_ACCESS_KEY', 'idni3vSg54jWli6HyX8bLe7F8ro682M6')
BUCKET = os.getenv('S3_BUCKET', 'camera-recordings')

# Initialize S3 client
s3_client = boto3.client(
    's3',
    endpoint_url=S3_ENDPOINT,
    aws_access_key_id=S3_ACCESS_KEY,
    aws_secret_access_key=S3_SECRET_KEY,
    config=Config(signature_version='s3v4'),
    region_name='us-east-1'
)

class PlaybackHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        
        if path == '/':
            self.serve_index()
        elif path == '/api/list':
            self.serve_list()
        elif path.startswith('/video/'):
            self.serve_video(path[7:])  # Remove '/video/' prefix
        else:
            self.send_error(404, "Not Found")
    
    def serve_index(self):
        """Serve HTML interface"""
        html = """
<!DOCTYPE html>
<html>
<head>
    <title>Camera Recordings</title>
    <style>
        body { font-family: system-ui; max-width: 1200px; margin: 40px auto; padding: 20px; }
        h1 { color: #333; }
        .camera { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 8px; }
        .camera h2 { margin-top: 0; color: #0066cc; }
        .recordings { display: flex; flex-wrap: wrap; gap: 10px; }
        .recording { padding: 8px 12px; background: #f5f5f5; border-radius: 4px; cursor: pointer; }
        .recording:hover { background: #e0e0e0; }
        video { max-width: 100%; margin: 20px 0; }
        .loading { color: #666; font-style: italic; }
        button { padding: 8px 16px; background: #0066cc; color: white; border: none; border-radius: 4px; cursor: pointer; }
        button:hover { background: #0052a3; }
    </style>
</head>
<body>
    <h1>🎥 Camera Recordings</h1>
    <div id="status" class="loading">Loading recordings...</div>
    <div id="cameras"></div>
    <div id="player" style="display:none;">
        <h2>Playback</h2>
        <video id="video" controls width="100%"></video>
        <button onclick="closePlayer()">Close</button>
    </div>
    
    <script>
        async function loadRecordings() {
            const response = await fetch('/api/list');
            const data = await response.json();
            
            const status = document.getElementById('status');
            status.textContent = `Found ${data.total_recordings} recordings across ${Object.keys(data.cameras).length} cameras`;
            
            const container = document.getElementById('cameras');
            container.innerHTML = '';
            
            for (const [cameraId, recordings] of Object.entries(data.cameras)) {
                const cameraDiv = document.createElement('div');
                cameraDiv.className = 'camera';
                cameraDiv.innerHTML = `
                    <h2>Camera: ${cameraId}</h2>
                    <p>${recordings.length} recordings</p>
                    <div class="recordings" id="${cameraId}-recordings"></div>
                `;
                container.appendChild(cameraDiv);
                
                const recordingsDiv = document.getElementById(`${cameraId}-recordings`);
                recordings.forEach(rec => {
                    const div = document.createElement('div');
                    div.className = 'recording';
                    div.textContent = `${rec.date} - ${rec.filename} (${rec.size})`;
                    div.onclick = () => playVideo(rec.key);
                    recordingsDiv.appendChild(div);
                });
            }
        }
        
        function playVideo(key) {
            const video = document.getElementById('video');
            const player = document.getElementById('player');
            video.src = `/video/${key}`;
            player.style.display = 'block';
            player.scrollIntoView({ behavior: 'smooth' });
        }
        
        function closePlayer() {
            const video = document.getElementById('video');
            const player = document.getElementById('player');
            video.pause();
            video.src = '';
            player.style.display = 'none';
        }
        
        loadRecordings();
        setInterval(loadRecordings, 30000); // Refresh every 30 seconds
    </script>
</body>
</html>
        """
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(html.encode())
    
    def serve_list(self):
        """List all recordings from S3"""
        try:
            result = s3_client.list_objects_v2(Bucket=BUCKET)
            
            cameras = {}
            total = 0
            
            if 'Contents' in result:
                for obj in result['Contents']:
                    key = obj['Key']
                    parts = key.split('/')
                    if len(parts) >= 3:  # camera-id/date/filename
                        camera_id = parts[0]
                        date = parts[1]
                        filename = parts[2]
                        size = obj['Size']
                        
                        if camera_id not in cameras:
                            cameras[camera_id] = []
                        
                        cameras[camera_id].append({
                            'key': key,
                            'date': date,
                            'filename': filename,
                            'size': f"{size / 1048576:.1f} MB"
                        })
                        total += 1
            
            response = {
                'cameras': cameras,
                'total_recordings': total
            }
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
        except Exception as e:
            self.send_error(500, f"Error listing recordings: {e}")
    
    def serve_video(self, s3_key):
        """Stream video from S3"""
        try:
            obj = s3_client.get_object(Bucket=BUCKET, Key=s3_key)
            
            self.send_response(200)
            self.send_header('Content-type', 'video/mp4')
            self.send_header('Content-Length', obj['ContentLength'])
            self.end_headers()
            
            # Stream the video
            for chunk in obj['Body'].iter_chunks(chunk_size=8192):
                self.wfile.write(chunk)
        except Exception as e:
            self.send_error(404, f"Video not found: {e}")
    
    def log_message(self, format, *args):
        """Custom log format"""
        print(f"[{self.log_date_time_string()}] {format % args}")

def main():
    port = int(os.getenv('PORT', '8000'))
    server = HTTPServer(('0.0.0.0', port), PlaybackHandler)
    print(f"🎬 Camera Playback Server running on http://localhost:{port}")
    print(f"   S3 Endpoint: {S3_ENDPOINT}")
    print(f"   Bucket: {BUCKET}")
    print(f"\nOpen http://localhost:{port} in your browser to view recordings")
    server.serve_forever()

if __name__ == '__main__':
    main()
