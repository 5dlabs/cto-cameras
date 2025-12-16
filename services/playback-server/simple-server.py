#!/usr/bin/env python3
"""Simple HTTP server to browse and play local camera recordings"""

import os
import json
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path

RECORDINGS_DIR = "/tmp/camera-recordings"

class RecordingHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=RECORDINGS_DIR, **kwargs)
    
    def do_GET(self):
        if self.path == '/':
            self.serve_index()
        elif self.path == '/api/list':
            self.serve_list()
        else:
            super().do_GET()
    
    def serve_index(self):
        html = """
<!DOCTYPE html>
<html>
<head>
    <title>Camera Recordings (Local)</title>
    <style>
        body { font-family: system-ui; max-width: 1400px; margin: 40px auto; padding: 20px; background: #f8f9fa; }
        h1 { color: #333; margin-bottom: 10px; }
        .subtitle { color: #666; margin-bottom: 30px; }
        .camera { margin: 20px 0; padding: 20px; background: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .camera h2 { margin-top: 0; color: #0066cc; }
        .recordings { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px; }
        .recording { padding: 12px; background: #f5f5f5; border-radius: 4px; cursor: pointer; transition: all 0.2s; }
        .recording:hover { background: #e0e0e0; transform: translateY(-2px); box-shadow: 0 2px 8px rgba(0,0,0,0.1); }
        .recording .name { font-weight: 500; color: #333; }
        .recording .size { color: #666; font-size: 0.9em; margin-top: 4px; }
        video { max-width: 100%; margin: 20px 0; border-radius: 8px; box-shadow: 0 4px 8px rgba(0,0,0,0.2); }
        button { padding: 10px 20px; background: #0066cc; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; }
        button:hover { background: #0052a3; }
        .live-indicator { display: inline-block; width: 10px; height: 10px; background: #ff0000; border-radius: 50%; animation: pulse 2s infinite; margin-right: 8px; }
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
        .stats { background: #e8f4ff; padding: 15px; border-radius: 8px; margin: 20px 0; }
        .stats h3 { margin-top: 0; color: #0066cc; }
    </style>
</head>
<body>
    <h1>🎥 Camera Recordings</h1>
    <div class="subtitle"><span class="live-indicator"></span>LIVE RECORDING - Updating every 10 seconds</div>
    <div id="stats" class="stats"></div>
    <div id="cameras"></div>
    <div id="player" style="display:none; margin-top: 30px; padding: 20px; background: white; border-radius: 8px;">
        <h2>Playback</h2>
        <video id="video" controls width="100%"></video>
        <button onclick="closePlayer()" style="margin-top: 10px;">Close</button>
    </div>
    
    <script>
        async function loadRecordings() {
            const response = await fetch('/api/list');
            const data = await response.json();
            
            // Update stats
            const stats = document.getElementById('stats');
            stats.innerHTML = `
                <h3>📊 Recording Statistics</h3>
                <p><strong>Total Recordings:</strong> ${data.total_recordings} segments</p>
                <p><strong>Total Size:</strong> ${data.total_size_mb.toFixed(1)} MB</p>
                <p><strong>Cameras Active:</strong> ${Object.keys(data.cameras).length}</p>
            `;
            
            const container = document.getElementById('cameras');
            container.innerHTML = '';
            
            for (const [cameraId, info] of Object.entries(data.cameras)) {
                const cameraDiv = document.createElement('div');
                cameraDiv.className = 'camera';
                cameraDiv.innerHTML = `
                    <h2>📹 ${cameraId}</h2>
                    <p>${info.recordings.length} recordings • ${info.total_size_mb.toFixed(1)} MB total</p>
                    <div class="recordings" id="${cameraId}-recordings"></div>
                `;
                container.appendChild(cameraDiv);
                
                const recordingsDiv = document.getElementById(`${cameraId}-recordings`);
                info.recordings.forEach(rec => {
                    const div = document.createElement('div');
                    div.className = 'recording';
                    div.innerHTML = `
                        <div class="name">${rec.filename}</div>
                        <div class="size">${rec.size_mb.toFixed(1)} MB • ${rec.modified}</div>
                    `;
                    div.onclick = () => playVideo(rec.path);
                    recordingsDiv.appendChild(div);
                });
            }
        }
        
        function playVideo(path) {
            const video = document.getElementById('video');
            const player = document.getElementById('player');
            video.src = path;
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
        setInterval(loadRecordings, 10000); // Refresh every 10 seconds
    </script>
</body>
</html>
        """
        self.send_response(200)
        self.send_header('Content-type', 'text/html')
        self.end_headers()
        self.wfile.write(html.encode())
    
    def serve_list(self):
        """List all recordings from local filesystem"""
        try:
            cameras = {}
            total_size = 0
            total_count = 0
            
            for camera_dir in Path(RECORDINGS_DIR).iterdir():
                if camera_dir.is_dir() and camera_dir.name.startswith('camera-'):
                    camera_id = camera_dir.name
                    recordings = []
                    camera_size = 0
                    
                    for video_file in camera_dir.glob('*.mp4'):
                        stat = video_file.stat()
                        size_mb = stat.st_size / 1048576
                        
                        recordings.append({
                            'filename': video_file.name,
                            'path': f'/{camera_id}/{video_file.name}',
                            'size_mb': size_mb,
                            'modified': video_file.stat().st_mtime
                        })
                        camera_size += size_mb
                        total_size += size_mb
                        total_count += 1
                    
                    # Sort by filename (which includes timestamp)
                    recordings.sort(key=lambda x: x['filename'], reverse=True)
                    
                    cameras[camera_id] = {
                        'recordings': recordings,
                        'total_size_mb': camera_size
                    }
            
            response = {
                'cameras': cameras,
                'total_recordings': total_count,
                'total_size_mb': total_size
            }
            
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(response).encode())
        except Exception as e:
            self.send_error(500, f"Error listing recordings: {e}")

def main():
    port = int(os.getenv('PORT', '8000'))
    server = HTTPServer(('0.0.0.0', port), RecordingHandler)
    print(f"🎬 Camera Playback Server running on http://localhost:{port}")
    print(f"   Serving recordings from: {RECORDINGS_DIR}")
    print(f"\nOpen http://localhost:{port} in your browser to view recordings")
    server.serve_forever()

if __name__ == '__main__':
    main()
