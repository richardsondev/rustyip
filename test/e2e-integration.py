#!/usr/bin/env python3

import http.server
import ssl
import socketserver
import subprocess
import time
import sys
import os
import threading
from typing import Any

class LoggingHTTPRequestHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format: str, *args: Any) -> None:
        timestamp = time.strftime('%Y-%m-%d %H:%M:%S')
        sys.stdout.write(f"[{timestamp}] {format % args}\n")
        sys.stdout.flush()

    def do_GET(self):
        print(f"📥 GET {self.path}")
        print(f"   Headers: {dict(self.headers)}")
        sys.stdout.flush()
        
        if self.path == '/ip.txt':
            # Return a fake IP address
            self.send_response(200)
            self.send_header('Content-type', 'text/plain')
            self.end_headers()
            self.wfile.write(b'192.168.1.100')
            print(f"✅ Served IP address: 192.168.1.100")
        else:
            self.send_response(404)
            self.send_header('Content-type', 'text/plain')
            self.end_headers()
            self.wfile.write(b'Not Found')
            print(f"❌ Path not found: {self.path}")
        sys.stdout.flush()

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)
        print(f"📤 POST {self.path}")
        print(f"   Headers: {dict(self.headers)}")
        print(f"   Data: {post_data.decode('utf-8', errors='ignore')}")
        sys.stdout.flush()
        
        self.send_response(200)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'OK')
        print(f"✅ POST request processed successfully")
        sys.stdout.flush()

def run_https_server():
    """Run HTTPS server in a separate thread"""
    try:
        PORT = 443
        httpd = socketserver.TCPServer(("", PORT), LoggingHTTPRequestHandler)
        
        # Create SSL context
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain('certs/server-cert.pem', 'certs/server-key.pem')
        
        httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
        
        print(f"🔒 HTTPS server started on port {PORT}")
        sys.stdout.flush()
        httpd.serve_forever()
    except Exception as e:
        print(f"❌ HTTPS server error: {e}")
        sys.stdout.flush()

def test_container():
    """Test the RustyIP binary directly"""
    try:
        # Generate random values for environment variables
        import secrets
        KEY = secrets.token_hex(100)
        TOKEN = secrets.token_hex(16) 
        HASH = secrets.token_hex(8)
        HOST = "localhost"
        SLEEP_DURATION = "1"
        
        print("🧪 Testing RustyIP binary with environment variables...")
        print(f"   KEY: {KEY}")
        print(f"   TOKEN: {TOKEN}")
        print(f"   HASH: {HASH}")
        print(f"   HOST: {HOST}")
        print(f"   SLEEP_DURATION: {SLEEP_DURATION}")
        sys.stdout.flush()
        
        # Set environment variables
        env = os.environ.copy()
        env.update({
            'KEY': KEY,
            'TOKEN': TOKEN,
            'HASH': HASH,
            'HOST': HOST,
            'SLEEP_DURATION': SLEEP_DURATION
        })
        # Run RustyIP binary directly
        print("🚀 Starting RustyIP binary...")
        rustyip_process = subprocess.Popen([
            '/usr/src/RustyIP/target/x86_64-unknown-linux-musl/release/RustyIP'
        ], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        
        # Let it run for a bit to make some requests
        print("⏳ Letting RustyIP run for 30 seconds...")
        time.sleep(30)
        
        # Stop the process
        print("🛑 Stopping RustyIP...")
        rustyip_process.terminate()
        try:
            rustyip_output, _ = rustyip_process.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            rustyip_process.kill()
            rustyip_output, _ = rustyip_process.communicate()
        
        print("📋 RustyIP output:")
        print(rustyip_output)
        
        return True
        
    except Exception as e:
        print(f"❌ RustyIP test error: {e}")
        import traceback
        traceback.print_exc()
        return False

def main():
    print("🏁 Starting RustyIP integration test")
    
    # Verify certificates exist
    if not all(os.path.exists(f) for f in ['certs/ca-cert.pem', 'certs/server-cert.pem', 'certs/server-key.pem']):
        print("❌ Certificate files not found. Run generate-test-certs.sh first.")
        sys.exit(1)
    
    # Start HTTPS server in background thread
    server_thread = threading.Thread(target=run_https_server, daemon=True)
    server_thread.start()
    
    # Wait for server to start
    time.sleep(3)
    
    # Test HTTPS server is responding
    try:
        result = subprocess.run(['curl', '-k', 'https://localhost/ip.txt'], 
                              capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            print(f"✅ HTTPS server responding: {result.stdout.strip()}")
        else:
            print(f"❌ HTTPS server not responding: {result.stderr}")
            sys.exit(1)
    except Exception as e:
        print(f"❌ Failed to test HTTPS server: {e}")
        sys.exit(1)

    print("🔗 HTTPS server is up and running!")
    sys.stdout.flush()

    # Test the binary
    success = test_container()
    
    if success:
        print("✅ Integration test completed successfully!")
        sys.exit(0)
    else:
        print("❌ Integration test failed!")
        sys.exit(1)

if __name__ == '__main__':
    main()
