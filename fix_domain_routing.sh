#!/bin/bash

# Fix script to properly set up domain-based reverse proxy
# This keeps your existing Rust service on port 80 and adds domain routing

set -e

VPS_IP="159.198.32.51"
VPS_USER="root"
DOMAIN="fuelcost_dashboard.blackshadow.software"
SERVICE_NAME="fuelcost_dashboard"
DASHBOARD_PORT="8890"
REMOTE_DIR="/var/www/fuelcost_dashboard"

echo "🔧 Setting up proper domain-based reverse proxy for $DOMAIN"
echo "📋 This will:"
echo "   • Keep your existing Rust service running on port 80"
echo "   • Add domain-based routing in Nginx"
echo "   • Make $DOMAIN work without port numbers"

# Step 1: Check what's currently on port 80
echo ""
echo "🔍 Step 1: Analyzing current port 80 configuration..."

ssh $VPS_USER@$VPS_IP << 'ENDSSH'

echo "📊 What's using port 80:"
netstat -tlnp | grep :80

echo ""
echo "📄 Current Nginx sites:"
ls -la /etc/nginx/sites-enabled/

echo ""
echo "🔍 Current Nginx configuration structure:"
nginx -T 2>/dev/null | grep -E "server_name|listen.*80|proxy_pass" | head -20

ENDSSH

echo ""
read -p "🤔 Do you want to proceed with adding domain routing? (y/n): " confirm

if [ "$confirm" != "y" ]; then
    echo "❌ Cancelled by user"
    exit 0
fi

# Step 2: Set up the dashboard service (static file server)
echo ""
echo "🔧 Step 2: Setting up dashboard static file server..."

ssh $VPS_USER@$VPS_IP << ENDSSH

# Create directory and upload will be handled separately
mkdir -p $REMOTE_DIR
chown -R www-data:www-data $REMOTE_DIR

# Find available port starting from 8890
echo "🔍 Finding available port..."
CURRENT_PORT=$DASHBOARD_PORT
while netstat -ln | grep -q ":$CURRENT_PORT "; do
    echo "⚠️ Port $CURRENT_PORT is busy, trying next port..."
    CURRENT_PORT=$((CURRENT_PORT + 1))
    # Safety check
    if [ $CURRENT_PORT -gt $((DASHBOARD_PORT + 50)) ]; then
        echo "❌ Could not find available port after checking 50 ports"
        exit 1
    fi
done
DASHBOARD_PORT=$CURRENT_PORT
echo "✅ Using port: $DASHBOARD_PORT"

# Stop any existing service first
systemctl stop ${SERVICE_NAME}.service 2>/dev/null || true

# Create Node.js static server (preferred)
if which node >/dev/null 2>&1; then
    echo "✅ Using Node.js for static server"
    
    cat > /usr/local/bin/${SERVICE_NAME}_server.js << 'EOF'
const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = $DASHBOARD_PORT;
const DIRECTORY = '$REMOTE_DIR';

const server = http.createServer((req, res) => {
    // CORS headers
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS, PUT, DELETE');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    
    if (req.method === 'OPTIONS') {
        res.writeHead(204);
        res.end();
        return;
    }
    
    let filePath = path.join(DIRECTORY, req.url === '/' ? 'index.html' : req.url);
    
    fs.readFile(filePath, (err, data) => {
        if (err) {
            res.writeHead(404);
            res.end('File not found');
            return;
        }
        
        const ext = path.extname(filePath);
        const contentType = ext === '.html' ? 'text/html' : 
                          ext === '.css' ? 'text/css' :
                          ext === '.js' ? 'application/javascript' : 'text/plain';
        
        res.writeHead(200, { 'Content-Type': contentType });
        res.end(data);
    });
});

server.listen(PORT, '127.0.0.1', () => {
    console.log(\`Dashboard server running at http://127.0.0.1:\${PORT}\`);
});
EOF

    SERVER_EXEC="node /usr/local/bin/${SERVICE_NAME}_server.js"
else
    echo "✅ Using Python for static server"
    
    cat > /usr/local/bin/${SERVICE_NAME}_server.py << 'EOF'
#!/usr/bin/env python3
import http.server
import socketserver
import os

PORT = $DASHBOARD_PORT
DIRECTORY = "$REMOTE_DIR"

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIRECTORY, **kwargs)
    
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        super().end_headers()

os.chdir(DIRECTORY)
with socketserver.TCPServer(("127.0.0.1", PORT), Handler) as httpd:
    print(f"Dashboard server running at http://127.0.0.1:{PORT}")
    httpd.serve_forever()
EOF

    chmod +x /usr/local/bin/${SERVICE_NAME}_server.py
    SERVER_EXEC="python3 /usr/local/bin/${SERVICE_NAME}_server.py"
fi

# Create systemd service with proper variable handling
if which node >/dev/null 2>&1; then
    ACTUAL_SERVER_EXEC="node /usr/local/bin/${SERVICE_NAME}_server.js"
else
    ACTUAL_SERVER_EXEC="python3 /usr/local/bin/${SERVICE_NAME}_server.py"
fi

cat > /etc/systemd/system/${SERVICE_NAME}.service << EOF
[Unit]
Description=${SERVICE_NAME} Dashboard Service
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=$REMOTE_DIR
ExecStart=\$ACTUAL_SERVER_EXEC
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable ${SERVICE_NAME}.service

ENDSSH

# Step 3: Upload the HTML file
echo ""
echo "📁 Step 3: Uploading dashboard files..."

scp index.html $VPS_USER@$VPS_IP:$REMOTE_DIR/
ssh $VPS_USER@$VPS_IP "chown www-data:www-data $REMOTE_DIR/index.html && chmod 644 $REMOTE_DIR/index.html"

# Step 4: Configure Nginx for domain-based routing
echo ""
echo "🌐 Step 4: Setting up domain-based Nginx configuration..."

ssh $VPS_USER@$VPS_IP << ENDSSH

# Create the domain-specific site configuration
cat > /etc/nginx/sites-available/$DOMAIN << 'EOF'
# Dashboard domain routing
server {
    listen 80;
    server_name $DOMAIN;
    
    # Proxy all requests to the dashboard service
    location / {
        proxy_pass http://127.0.0.1:$DASHBOARD_PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_cache_bypass \$http_upgrade;
        proxy_read_timeout 86400;
    }
    
    # Logs
    access_log /var/log/nginx/${SERVICE_NAME}_access.log;
    error_log /var/log/nginx/${SERVICE_NAME}_error.log;
}
EOF

# Enable the site
ln -sf /etc/nginx/sites-available/$DOMAIN /etc/nginx/sites-enabled/

# Test configuration
echo "🔍 Testing Nginx configuration..."
if nginx -t; then
    echo "✅ Nginx configuration is valid"
    systemctl reload nginx
    echo "✅ Nginx reloaded"
else
    echo "❌ Nginx configuration error"
    exit 1
fi

ENDSSH

# Step 5: Start the dashboard service
echo ""
echo "🚀 Step 5: Starting dashboard service..."

ssh $VPS_USER@$VPS_IP << ENDSSH

# Start the dashboard service
systemctl start ${SERVICE_NAME}.service

sleep 5

# Check if service is running with detailed diagnostics
echo "📊 Service Status:"
systemctl status ${SERVICE_NAME}.service --no-pager -l

echo ""
if systemctl is-active --quiet ${SERVICE_NAME}.service; then
    echo "✅ Dashboard service is running"
else
    echo "❌ Dashboard service failed to start"
    echo "🔍 Service logs:"
    journalctl -u ${SERVICE_NAME}.service --no-pager -n 10
    echo "🔍 Checking file permissions:"
    ls -la $REMOTE_DIR/
    ls -la /usr/local/bin/${SERVICE_NAME}_server.*
    exit 1
fi

ENDSSH

# Step 6: Test the setup
echo ""
echo "🧪 Step 6: Testing the complete setup..."

ssh $VPS_USER@$VPS_IP << ENDSSH

echo "Testing dashboard service directly..."
DIRECT_STATUS=\$(curl -s -o /dev/null -w '%{http_code}' http://localhost:$DASHBOARD_PORT/ || echo 'FAILED')
echo "Direct service test: \$DIRECT_STATUS"

echo "Testing through Nginx reverse proxy..."
PROXY_STATUS=\$(curl -s -o /dev/null -w '%{http_code}' -H "Host: $DOMAIN" http://localhost/ || echo 'FAILED')
echo "Reverse proxy test: \$PROXY_STATUS"

echo "Testing HTML content..."
if curl -s -H "Host: $DOMAIN" http://localhost/ | head -n 3 | grep -q html; then
    echo "✅ HTML content is accessible"
else
    echo "❌ HTML content test failed"
fi

ENDSSH

echo ""
echo "🎉 Setup complete!"
echo ""
echo "📋 Summary:"
echo "  • Dashboard service running on: http://127.0.0.1:$DASHBOARD_PORT"
echo "  • Nginx reverse proxy configured for: $DOMAIN"
echo "  • Your existing Rust service should still work on its domain"
echo ""
echo "🔗 Test your dashboard at: http://$DOMAIN"
echo ""
echo "⚠️  Make sure your DNS A record points $DOMAIN to $VPS_IP"